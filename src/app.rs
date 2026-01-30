use std::cell::RefCell;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

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

const NUM_COMPONENTS: usize = 3; // package_input, packages_table, package_info

pub(crate) struct App {
    tui: Tui,
    package_input: PackageInput,
    components: Vec<Box<dyn Component>>,
    pacman: Pacman,
    should_exit: bool,
    runtime: Runtime,
    aur_sender: UnboundedSender<Vec<Package>>,
    aur_receiver: UnboundedReceiver<Vec<Package>>,
    focused: usize, // 0 = package_input, 1 = packages_table, 2 = package_info
}

impl App {
    pub(crate) fn new() -> eyre::Result<Self> {
        let tui = Tui::new()?;
        let should_exit = Default::default();
        let pacman = Pacman::new()?;

        // Create tokio runtime for async tasks
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        let (aur_sender, aur_receiver) = tokio::sync::mpsc::unbounded_channel();

        tracing::debug!("tokio runtime created for async AUR searches");

        let mut package_input = PackageInput::default();
        package_input.set_active(true); // Start with focus on input

        let mut packages_table = PackagesTable::default();
        packages_table.set_active(false);

        let mut package_info = PackageInfo::default();
        package_info.set_active(false);

        Ok(Self {
            tui,
            package_input,
            components: vec![
                Box::new(packages_table),
                Box::new(package_info),
            ],
            pacman,
            should_exit,
            runtime,
            aur_sender,
            aur_receiver,
            focused: 0,
        })
    }

    fn set_focus(&mut self, new_focus: usize) {
        tracing::debug!(current = self.focused, new = new_focus, "set_focus called");

        if new_focus == self.focused || new_focus >= NUM_COMPONENTS {
            tracing::debug!("set_focus: no change needed");
            return;
        }

        // Deactivate current
        match self.focused {
            0 => self.package_input.set_active(false),
            n => self.components[n - 1].set_active(false),
        }

        self.focused = new_focus;

        // Activate new
        match self.focused {
            0 => self.package_input.set_active(true),
            n => self.components[n - 1].set_active(true),
        }

        tracing::info!(focused = self.focused, "focus changed");
    }

    fn cycle_focus(&mut self) {
        let new_focus = (self.focused + 1) % NUM_COMPONENTS;
        self.set_focus(new_focus);
    }

    pub(crate) fn run(&mut self) -> eyre::Result<()> {
        tracing::debug!("entering TUI mode");
        Tui::enter()?;

        while !self.should_exit {
            self.render()?;
            let actions = self.handle_events()?;
            self.handle_actions(&actions)?;

            // Check if AUR debounce timer elapsed
            if let Some(query) = self.package_input.should_search_aur() {
                self.start_aur_search(&query);
            }

            // Check for AUR results from async task
            if let Ok(aur_packages) = self.aur_receiver.try_recv() {
                tracing::debug!(count = aur_packages.len(), "received AUR packages from async task");
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

        // Handle focus switching
        use ratatui::crossterm::event::KeyModifiers;
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.cycle_focus();
                return Ok(Vec::new());
            }
            // Alt+j - focus down (input -> packages)
            (KeyCode::Char('j'), KeyModifiers::ALT) => {
                if self.focused < 1 {
                    self.set_focus(1);
                }
                return Ok(Vec::new());
            }
            // Alt+k - focus up (packages -> input, info -> input)
            (KeyCode::Char('k'), KeyModifiers::ALT) => {
                if self.focused > 0 {
                    self.set_focus(0);
                }
                return Ok(Vec::new());
            }
            // Alt+l - focus right (to info panel)
            (KeyCode::Char('l'), KeyModifiers::ALT) => {
                if self.focused != 2 {
                    self.set_focus(2);
                }
                return Ok(Vec::new());
            }
            // Alt+h - focus left (from info to packages)
            (KeyCode::Char('h'), KeyModifiers::ALT) => {
                if self.focused == 2 {
                    self.set_focus(1);
                }
                return Ok(Vec::new());
            }
            _ => {}
        }

        let mut actions = Vec::new();

        // Only send key events to the focused component
        match self.focused {
            0 => {
                if let Some(component_actions) = self.package_input.handle_key_event(key_event)? {
                    actions.extend(component_actions);
                }
            }
            n => {
                if let Some(component_actions) = self.components[n - 1].handle_key_event(key_event)? {
                    actions.extend(component_actions);
                }
            }
        }

        Ok(actions)
    }

    fn start_aur_search(&self, query: &str) {
        let sender = self.aur_sender.clone();
        let installed = self.pacman.installed_packages().clone();
        let query = query.to_string();

        // Spawn async task on tokio runtime
        self.runtime.spawn(async move {
            tracing::debug!(query = %query, "starting async AUR search");
            match aur::search(&query, &installed).await {
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
                tracing::debug!(query, "searching for package (pacman only, instant)");

                // Pacman search only (fast, local) - AUR is triggered separately after debounce
                let packages = self.pacman.search_package(query)?;
                tracing::debug!(count = packages.len(), "found pacman packages");

                // Auto-select first package if available
                if let Some(first) = packages.first() {
                    events.push(crate::event::Event::PackageSelected(Box::new(first.clone())));
                }
                events.push(crate::event::Event::FoundPackages(packages));
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
