use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        self,
        event::{self, KeyCode, KeyEvent},
    },
    Terminal,
};

use crate::tui::TUI;
use crate::{action::Action, components::home::HomeComponent};
use crate::{components::Component, pacman};

use std::{io, time};

pub struct App {
    tui: TUI,
    components: Vec<Box<dyn Component>>,
    exit: bool,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let writer = io::stdout();
        let backend = CrosstermBackend::new(writer);
        let terminal = Terminal::new(backend)?;
        let tui = TUI::new(terminal)
            .with_raw_mode()?
            .with_alternate_screen()?;

        Ok(Self {
            tui,
            components: vec![Box::new(HomeComponent::default())],
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

    fn handle_events(&mut self) -> io::Result<Vec<Option<Action>>> {
        let mut actions = Vec::new();

        if event::poll(time::Duration::from_millis(16))? {
            match event::read()? {
                crossterm::event::Event::Key(key_event) => {
                    let mut components_actions = self.handle_key_event(key_event);
                    actions.append(&mut components_actions);
                }
                _ => {}
            };
        }

        Ok(actions)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Vec<Option<Action>> {
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

    fn handle_actions(&mut self, actions: Vec<Option<Action>>) {
        for action in actions {
            if let Some(action) = action {
                self.handle_action(action);
            }
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::InstallPackage(name) => {
                self.tui
                    .suspend(|| {
                        pacman::install(&name);
                    })
                    .unwrap();
            }
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
