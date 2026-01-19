use std::cell::RefCell;

use crate::action::Action;
use crate::components::package_info::PackageInfo;
use crate::components::packages_table::PackagesTable;
use crate::components::{package_input::PackageInput, Component};
use crate::pacman::{self, Pacman};
use crate::tui::TUI;

use color_eyre::eyre;
use ratatui::crossterm;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};

pub(crate) struct App {
    tui: TUI,
    components: Vec<Box<dyn Component>>,
    pacman: Pacman,
    should_exit: bool,
}

impl App {
    pub(crate) fn new() -> eyre::Result<Self> {
        let tui = TUI::new()?;
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
        TUI::enter()?;

        while !self.should_exit {
            self.render()?;
            let actions = self.handle_events()?;
            self.handle_actions(&actions)?;
        }

        tracing::debug!("exiting TUI mode");
        TUI::exit()?;

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

        match crossterm::event::read()? {
            Event::Key(key_event) => {
                let component_actions = self.handle_key_event(&key_event)?;
                actions.extend(component_actions);
            }
            _ => {}
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

    fn handle_actions(&mut self, actions: &Vec<Action>) -> eyre::Result<()> {
        let mut events = Vec::new();

        for action in actions {
            let app_events = self.handle_action(&action)?;
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
            Action::SearchPackage(package_name) => {
                tracing::debug!(package_name, "searching for package");
                let packages = self.pacman.search_package(package_name)?;
                tracing::debug!(count = packages.len(), "found packages");
                events.push(crate::event::Event::FoundPackages(packages));
            }
            Action::InstallPackage(package_name) => {
                tracing::info!(package_name, "installing package");
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = pacman::install_package(package_name)?;
                    if status.success() {
                        tracing::info!(package_name, "package installed successfully");
                        events.push(crate::event::Event::PackageInstalled(package_name.clone()));
                    } else {
                        tracing::warn!(package_name, code = ?status.code(), "package installation failed");
                    }
                    Ok(())
                })?;
            }
            Action::UpdateInstallPackage(package_name) => {
                tracing::info!(package_name, "updating and installing package");
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = pacman::update_install_package(package_name)?;
                    if status.success() {
                        tracing::info!(package_name, "package updated and installed successfully");
                        events.push(crate::event::Event::PackageInstalled(package_name.clone()));
                    } else {
                        tracing::warn!(package_name, code = ?status.code(), "package update/install failed");
                    }
                    Ok(())
                })?;
            }
            Action::RemovePackage(package_name) => {
                tracing::info!(package_name, "removing package");
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = pacman::remove_package(package_name)?;
                    if status.success() {
                        tracing::info!(package_name, "package removed successfully");
                        events.push(crate::event::Event::PackageRemoved(package_name.clone()));
                    } else {
                        tracing::warn!(package_name, code = ?status.code(), "package removal failed");
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
