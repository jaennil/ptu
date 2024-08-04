use std::process;

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
        TUI::enter()?;

        while !self.should_exit {
            self.render()?;
            let actions = self.handle_events()?;
            self.handle_actions(&actions)?;
        }

        TUI::exit()?;

        Ok(())
    }

    fn render(&mut self) -> eyre::Result<()> {
        self.tui.draw(|frame| {
            for component in self.components.iter_mut() {
                let result = component.draw(frame, &frame.size());
                if result.is_err() {
                    process::exit(1);
                }
            }
        })?;

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
                let packages = self.pacman.search_package(package_name)?;
                events.push(crate::event::Event::FoundPackages(packages));
                Ok(())
            }
            Action::InstallPackage(package_name) => self
                .tui
                .suspend(|| -> eyre::Result<()> { pacman::install_package(package_name) }),
            Action::SelectPackage(package) => {
                events.push(crate::event::Event::PackageSelected(package.clone()));
                Ok(())
            }
        }?;

        Ok(events)
    }
}
