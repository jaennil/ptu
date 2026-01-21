use std::cell::RefCell;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::action::Action;
use crate::aur;
use crate::components::package_info::PackageInfo;
use crate::components::packages_table::PackagesTable;
use crate::components::{package_input::PackageInput, Component};
use crate::pacman::{self, Package, Pacman};
use crate::tui::Tui;

use color_eyre::eyre;
use ratatui::crossterm;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};

pub(crate) struct App {
    tui: Tui,
    package_input: PackageInput,
    components: Vec<Box<dyn Component>>,
    pacman: Pacman,
    should_exit: bool,
    aur_sender: Sender<Vec<Package>>,
    aur_receiver: Receiver<Vec<Package>>,
}

impl App {
    pub(crate) fn new() -> eyre::Result<Self> {
        let tui = Tui::new()?;
        let should_exit = Default::default();
        let pacman = Pacman::new()?;
        let (aur_sender, aur_receiver) = mpsc::channel();

        Ok(Self {
            tui,
            package_input: PackageInput::default(),
            components: vec![
                Box::new(PackagesTable::default()),
                Box::new(PackageInfo::default()),
            ],
            pacman,
            should_exit,
            aur_sender,
            aur_receiver,
        })
    }

    pub(crate) fn run(&mut self) -> eyre::Result<()> {
        tracing::debug!("entering TUI mode");
        Tui::enter()?;

        while !self.should_exit {
            self.render()?;
            let mut actions = self.handle_events()?;

            // Check if debounce timer elapsed for search
            if let Some(query) = self.package_input.should_search() {
                actions.push(Action::SearchPackage(query));
            }

            self.handle_actions(&actions)?;

            // Check for AUR results from background thread
            if let Ok(aur_packages) = self.aur_receiver.try_recv() {
                tracing::debug!(count = aur_packages.len(), "received AUR packages from background thread");
                let event = crate::event::Event::AurPackagesFound(aur_packages);
                for component in self.components.iter_mut() {
                    component.update(&event)?;
                }
            }
        }

        tracing::debug!("exiting TUI mode");
        Tui::exit()?;

        Ok(())
    }

    fn render(&mut self) -> eyre::Result<()> {
        let render_error: RefCell<Option<eyre::Report>> = RefCell::new(None);

        self.tui.draw(|frame| {
            // Draw package input first
            if let Err(e) = self.package_input.draw(frame, &frame.area()) {
                *render_error.borrow_mut() = Some(e);
                return;
            }
            // Draw other components
            for component in self.components.iter_mut() {
                if let Err(e) = component.draw(frame, &frame.area()) {
                    *render_error.borrow_mut() = Some(e);
                    return;
                }
            }
        })?;

        if let Some(e) = render_error.into_inner() {
            return Err(e);
        }

        Ok(())
    }

    fn handle_events(&mut self) -> eyre::Result<Vec<Action>> {
        let mut actions = Vec::new();

        // Poll with 16ms timeout for ~60fps responsiveness
        if crossterm::event::poll(std::time::Duration::from_millis(16))?
            && let Event::Key(key_event) = crossterm::event::read()?
        {
            let component_actions = self.handle_key_event(&key_event)?;
            actions.extend(component_actions);
        }

        Ok(actions)
    }

    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Vec<Action>> {
        if key_event.code == KeyCode::Esc {
            self.should_exit = true;
        }

        let mut actions = Vec::new();

        // Handle package input
        if let Some(component_actions) = self.package_input.handle_key_event(key_event)? {
            actions.extend(component_actions);
        }

        // Handle other components
        for component in self.components.iter_mut() {
            let component_actions = component.handle_key_event(key_event)?;
            if let Some(component_actions) = component_actions {
                actions.extend(component_actions);
            }
        }

        Ok(actions)
    }

    fn handle_actions(&mut self, actions: &[Action]) -> eyre::Result<()> {
        let mut events = Vec::new();

        for action in actions {
            let app_events = self.handle_action(action)?;
            events.extend(app_events);
        }

        for component in self.components.iter_mut() {
            for event in &events {
                component.update(event)?;
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: &Action) -> eyre::Result<Vec<crate::event::Event>> {
        let mut events = Vec::new();

        match action {
            Action::SearchPackage(query) => {
                tracing::debug!(query, "searching for package");

                // Pacman search (fast, local)
                let packages = self.pacman.search_package(query)?;
                tracing::debug!(count = packages.len(), "found pacman packages");
                events.push(crate::event::Event::FoundPackages(packages));

                // AUR search in background thread
                let sender = self.aur_sender.clone();
                let installed = self.pacman.installed_packages().clone();
                let query = query.clone();
                std::thread::spawn(move || {
                    tracing::debug!(query = %query, "starting AUR search in background");
                    match aur::search(&query, &installed) {
                        Ok(aur_packages) => {
                            tracing::debug!(count = aur_packages.len(), query = %query, "AUR search completed");
                            if let Err(e) = sender.send(aur_packages) {
                                tracing::warn!(%e, "failed to send AUR results");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(%e, "AUR search failed");
                        }
                    }
                });
            }
            Action::InstallPackage { name, source } => {
                tracing::info!(name, source, "installing package");
                let mut success = false;
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = if source == "aur" {
                        aur::install(name)?
                    } else {
                        pacman::install_package(name)?
                    };
                    success = status.success();
                    Ok(())
                })?;
                if success {
                    tracing::info!(name, "package installed successfully");
                    self.pacman.mark_installed(name);
                    events.push(crate::event::Event::PackageInstalled(name.clone()));
                } else {
                    let error = "install failed".to_string();
                    tracing::warn!(name, %error, "package installation failed");
                    events.push(crate::event::Event::OperationFailed {
                        package: name.clone(),
                        error,
                    });
                }
            }
            Action::UpdateInstallPackage { name, source } => {
                tracing::info!(name, source, "updating and installing package");
                let mut success = false;
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = if source == "aur" {
                        aur::install(name)?
                    } else {
                        pacman::update_install_package(name)?
                    };
                    success = status.success();
                    Ok(())
                })?;
                if success {
                    tracing::info!(name, "package updated and installed successfully");
                    self.pacman.mark_installed(name);
                    events.push(crate::event::Event::PackageInstalled(name.clone()));
                } else {
                    let error = "update/install failed".to_string();
                    tracing::warn!(name, %error, "package update/install failed");
                    events.push(crate::event::Event::OperationFailed {
                        package: name.clone(),
                        error,
                    });
                }
            }
            Action::RemovePackage { name, source } => {
                tracing::info!(name, source, "removing package");
                let mut success = false;
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = if source == "aur" {
                        aur::remove(name)?
                    } else {
                        pacman::remove_package(name)?
                    };
                    success = status.success();
                    Ok(())
                })?;
                if success {
                    tracing::info!(name, "package removed successfully");
                    self.pacman.mark_removed(name);
                    events.push(crate::event::Event::PackageRemoved(name.clone()));
                } else {
                    let error = "remove failed".to_string();
                    tracing::warn!(name, %error, "package removal failed");
                    events.push(crate::event::Event::OperationFailed {
                        package: name.clone(),
                        error,
                    });
                }
            }
            Action::SelectPackage(package) => {
                tracing::debug!(package_name = package.name, "package selected");
                events.push(crate::event::Event::PackageSelected(package.clone()));
            }
        };

        Ok(events)
    }
}
