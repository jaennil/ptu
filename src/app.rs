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
    exit: bool,
    components: Vec<Box<dyn Component>>,
    // pacman: Pacman,
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
            self.handle_events()?;
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

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(time::Duration::from_millis(16))? {
            match event::read()? {
                crossterm::event::Event::Key(key_event) => self.handle_key_event(key_event),
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.code == KeyCode::Esc {
            self.exit();
            return;
        }

        let mut components_actions = Vec::new();
        for component in self.components.iter_mut() {
            let mut actions = component.handle_key_event(key_event).unwrap();
            components_actions.append(&mut actions);
        }
        for action in components_actions {
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
