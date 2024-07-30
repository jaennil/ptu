use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent};

use crate::{
    action::Action,
    components::{home::HomeComponent, search::PackageSearch},
    pacman::Pacman,
};
use crate::{components::table::PackagesTable, tui::TUI};
use crate::{components::Component, pacman};

use std::io;

pub struct App {
    tui: TUI,
    components: Vec<Box<dyn Component>>,
    pacman: Pacman,
    exit: bool,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let mut tui = TUI::new()?;
        tui.init()?;

        let pacman = match Pacman::new() {
            Ok(pacman) => pacman,
            Err(error) => return Err(io::Error::new(io::ErrorKind::Other, error.to_string())),
        };

        Ok(Self {
            tui,
            components: vec![
                Box::new(HomeComponent::default()),
                Box::new(PackageSearch::default()),
                Box::new(PackagesTable::default()),
            ],
            pacman,
            exit: Default::default(),
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        while !self.exit {
            self.render()?;
            let actions = self.handle_events()?;
            self.handle_actions(&actions);
        }

        self.tui.restore()
    }

    fn render(&mut self) -> io::Result<()> {
        self.tui.draw(|frame| {
            for component in self.components.iter_mut() {
                if component.draw(frame, frame.size()).is_err() {
                    // TODO: log error
                    std::process::exit(1);
                }
            }
        })?;

        Ok(())
    }

    fn handle_events(&mut self) -> io::Result<Vec<Action>> {
        let mut actions = Vec::new();

        match event::read()? {
            Event::Key(key_event) => {
                let mut components_actions = self.handle_key_event(&key_event)?;
                actions.append(&mut components_actions);
            }
            _ => {}
        };

        Ok(actions)
    }

    fn handle_key_event(&mut self, key_event: &KeyEvent) -> io::Result<Vec<Action>> {
        let mut actions = Vec::new();

        if key_event.code == KeyCode::Esc {
            self.exit();
            return Ok(actions);
        }

        for component in self.components.iter_mut() {
            let component_actions = component.handle_key_event(key_event)?;
            actions.extend(component_actions);
        }

        Ok(actions)
    }

    fn handle_actions(&mut self, actions: &Vec<Action>) -> io::Result<()> {
        let mut app_actions = actions.clone();

        for action in actions {
            let new_actions = self.handle_action(&action)?;
            app_actions.extend(new_actions);
        }

        for component in self.components.iter_mut() {
            for action in &app_actions {
                component.update(action);
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: &Action) -> io::Result<Vec<Action>> {
        let mut actions = Vec::new();

        match action {
            Action::InstallPackage(name) => self.tui.suspend(|| {
                pacman::install(name).unwrap();
            })?,
            Action::SearchPackage(package_name) => {
                let packages = self.pacman.search(package_name);
                actions.push(Action::FoundPackages(packages));
            }
            _ => {}
        }

        Ok(actions)
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
