use std::cell::RefCell;

use crate::action::Action;
use crate::aur;
use crate::components::package_info::PackageInfo;
use crate::components::packages_table::PackagesTable;
use crate::components::{package_input::PackageInput, Component};
use crate::pacman::{self, Pacman};
use crate::tui::Tui;

use color_eyre::eyre;
use ratatui::crossterm;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};

pub(crate) struct App {
    tui: Tui,
    components: Vec<Box<dyn Component>>,
    pacman: Pacman,
    should_exit: bool,
}

impl App {
    pub(crate) fn new() -> eyre::Result<Self> {
        let tui = Tui::new()?;
        let should_exit = Default::default();
        let pacman = Pacman::new()?;

        Ok(Self {
            tui,
            components: vec![
                Box::new(PackageInput::default()),
                Box::new(PackagesTable::default()),
                Box::new(PackageInfo::default()),
            ],
            pacman,
            should_exit,
        })
    }

    pub(crate) fn run(&mut self) -> eyre::Result<()> {
        tracing::debug!("entering TUI mode");
        Tui::enter()?;

        while !self.should_exit {
            self.render()?;
            let actions = self.handle_events()?;
            self.handle_actions(&actions)?;
        }

        tracing::debug!("exiting TUI mode");
        Tui::exit()?;

        Ok(())
    }

    fn render(&mut self) -> eyre::Result<()> {
        let render_error: RefCell<Option<eyre::Report>> = RefCell::new(None);

        self.tui.draw(|frame| {
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

        if let Event::Key(key_event) = crossterm::event::read()? {
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
                let mut packages = self.pacman.search_package(query)?;
                tracing::debug!(count = packages.len(), "found pacman packages");

                let installed = self.pacman.installed_packages();
                match aur::search(query, &installed) {
                    Ok(aur_packages) => {
                        tracing::debug!(count = aur_packages.len(), "found AUR packages");
                        packages.extend(aur_packages);
                    }
                    Err(e) => {
                        tracing::warn!(%e, "AUR search failed");
                    }
                }

                events.push(crate::event::Event::FoundPackages(packages));
            }
            Action::InstallPackage { name, source } => {
                tracing::info!(name, source, "installing package");
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = if source == "aur" {
                        aur::install(name)?
                    } else {
                        pacman::install_package(name)?
                    };
                    if status.success() {
                        tracing::info!(name, "package installed successfully");
                        events.push(crate::event::Event::PackageInstalled(name.clone()));
                    } else {
                        let error = format!("install exited with code {:?}", status.code());
                        tracing::warn!(name, %error, "package installation failed");
                        events.push(crate::event::Event::OperationFailed {
                            package: name.clone(),
                            error,
                        });
                    }
                    Ok(())
                })?;
            }
            Action::UpdateInstallPackage { name, source } => {
                tracing::info!(name, source, "updating and installing package");
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = if source == "aur" {
                        aur::install(name)?
                    } else {
                        pacman::update_install_package(name)?
                    };
                    if status.success() {
                        tracing::info!(name, "package updated and installed successfully");
                        events.push(crate::event::Event::PackageInstalled(name.clone()));
                    } else {
                        let error = format!("update/install exited with code {:?}", status.code());
                        tracing::warn!(name, %error, "package update/install failed");
                        events.push(crate::event::Event::OperationFailed {
                            package: name.clone(),
                            error,
                        });
                    }
                    Ok(())
                })?;
            }
            Action::RemovePackage { name, source } => {
                tracing::info!(name, source, "removing package");
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = if source == "aur" {
                        aur::remove(name)?
                    } else {
                        pacman::remove_package(name)?
                    };
                    if status.success() {
                        tracing::info!(name, "package removed successfully");
                        events.push(crate::event::Event::PackageRemoved(name.clone()));
                    } else {
                        let error = format!("remove exited with code {:?}", status.code());
                        tracing::warn!(name, %error, "package removal failed");
                        events.push(crate::event::Event::OperationFailed {
                            package: name.clone(),
                            error,
                        });
                    }
                    Ok(())
                })?;
            }
            Action::SelectPackage(package) => {
                tracing::debug!(package_name = package.name, "package selected");
                events.push(crate::event::Event::PackageSelected(package.clone()));
            }
        };

        Ok(events)
    }
}
