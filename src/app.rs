use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        self,
        event::{self, KeyCode, KeyEvent},
    }
};

use crate::components::Component;
use crate::components::home::HomeComponent;
use crate::tui::TUI;
use std::{io, time};

pub struct App {
    tui: TUI,
    exit: bool,
    // events: Vec<Event>,
    components: Vec<Box<dyn Component>>,
    // pacman: Pacman,
    // package_input: PackageSearch,
    // packages_table: PackagesTable,
    // mode: Mode,
}
//
// enum Event {
//     Key(KeyEvent),
// }

// #[derive(Default)]
// enum Mode {
//     #[default]
//     SearchPackage,
//     NavigatePackages,
// }
//
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
            // events: Default::default(),
            // mode: Mode::default(),
            // exit: false,
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
        // self.events.push(Event::Key(key_event));
        for component in self.components.iter_mut() {
            component.handle_key_event(key_event);
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

// match key_event {
//     KeyEvent {
//         kind: KeyEventKind::Press,
//         code,
//         modifiers,
//         ..
//     } => {
//
//         match modifiers {
//         KeyModifiers::CONTROL => match code {
//             // KeyCode::Char('j') => self.set_mode(Mode::NavigatePackages),
//             // KeyCode::Char('k') => self.set_mode(Mode::SearchPackage),
//             _ => {}
//         },
//         KeyModifiers::NONE => match code {
//             KeyCode::Esc => self.exit(),
//             _ => match self.mode {
//                         Mode::SearchPackage => {
//                             self.package_input.handle_key_event(key_event);
//                             match code {
//                                 KeyCode::Backspace | KeyCode::Char(_) => {
//                                     self.packages_table.packages =
//                                         self.pacman.search(&self.package_input.text);
//                                     self.packages_table.reset();
//                                 }
//                                 _ => {}
//                             }
//                         }
//                         Mode::NavigatePackages => {
//                             self.packages_table.handle_key_event(key_event);
//                             match code {
//                                 KeyCode::Char('i') => {
//                                     self.tui.suspend(|| {
//                                         pacman::install(
//                                             &self
//                                                 .packages_table
//                                                 .packages
//                                                 .get(self.packages_table.state.selected().unwrap())
//                                                 .unwrap()
//                                                 .name,
//                                         );
//                                     });
//                                 }
//                                 _ => {}
//                             }
//                         }
//                     },
//         },
//         _ => {}
//     },
//     }
//     _ => {}
// }
//
//
//
//
// }
//
// // impl App {
// //     fn set_mode(&mut self, mode: Mode) {
// //         match mode {
// //             Mode::SearchPackage => {
// //                 self.mode = Mode::SearchPackage;
// //                 self.package_input.active = true;
// //                 self.packages_table.active = false;
// //             }
// //             Mode::NavigatePackages => {
// //                 self.mode = Mode::NavigatePackages;
// //                 self.packages_table.active = true;
// //                 self.package_input.active = false;
// //             }
// //         }
// //     }
// // }
