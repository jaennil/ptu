use ratatui::crossterm::{
    self,
    event::{self, KeyCode, KeyEvent},
};

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

        Ok(Self {
            tui,
            components: vec![
                Box::new(HomeComponent::default()),
                Box::new(PackageSearch::default()),
                Box::new(PackagesTable::default()),
            ],
            pacman: Default::default(),
            exit: Default::default(),
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        while !self.exit {
            self.render()?;
            let actions = self.handle_events()?;
            self.handle_actions(actions);
        }
        self.tui.restore()
    }

    fn render(&mut self) -> io::Result<()> {
        self.tui.draw(|frame| {
            for component in self.components.iter_mut() {
                if let Err(_) = component.draw(frame, frame.size()) {
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
            crossterm::event::Event::Key(key_event) => {
                let mut components_actions = self.handle_key_event(key_event);
                actions.append(&mut components_actions);
            }
            _ => {}
        };

        Ok(actions)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Vec<Action> {
        let mut actions = Vec::new();

        if key_event.code == KeyCode::Esc {
            self.exit();
            return actions;
        }

        for component in self.components.iter_mut() {
            let mut component_actions = component.handle_key_event(key_event).unwrap();
            actions.append(&mut component_actions);
        }

        actions
    }

    fn handle_actions(&mut self, actions: Vec<Action>) {
        let mut na = actions.clone();
        for action in actions {
            if let Some(act) = self.handle_action(&action) {
                na.push(act);
            }
        }

        for action in na {
            for component in self.components.iter_mut() {
                component.update(&action);
            }
        }
    }

    fn handle_action(&mut self, action: &Action) -> Option<Action> {
        let mut new_action = None;

        match action {
            Action::InstallPackage(name) => {
                self.tui
                    .suspend(|| {
                        pacman::install(&name);
                    })
                    .unwrap();
            }
            Action::SearchPackage(package_name) => {
                new_action = Some(Action::FoundPackages(self.pacman.search(package_name)));
            }
            _ => {}
        }

        new_action
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
