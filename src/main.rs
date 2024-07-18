use std::io::{self, stdout};

use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    layout::{Constraint, Layout},
    Frame,
};

mod pacman;
mod search;
mod table;
mod theme;
mod tui;

use pacman::Pacman;
use search::PackageSearch;
use table::PackagesTable;

fn main() -> io::Result<()> {
    let mut terminal = tui::init()?;
    let mut app = App::default();
    let result = app.run(&mut terminal);
    tui::restore()?;
    result
}

#[derive(Default)]
enum Mode {
    #[default]
    Search,
    Table,
}

#[derive(Default)]
struct App {
    pacman: Pacman,
    search: PackageSearch,
    table: PackagesTable,
    mode: Mode,
    exit: bool,
}

impl App {
    fn run(&mut self, terminal: &mut tui::Terminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events(terminal)?;
        }
        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(frame.size());
        self.search.render(frame, layout[0]);
        self.table.render(frame, layout[1]);
    }

    fn handle_events(&mut self, terminal: &mut tui::Terminal) -> io::Result<()> {
        match event::read()? {
            // TODO: move key event kind check inside handle_key_event
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event, terminal)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent, terminal: &mut tui::Terminal) {
        if key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('j') {
            self.mode = Mode::Table;
            self.table.active = true;
            self.search.active = false;
            return;
        } else if key_event.modifiers == KeyModifiers::CONTROL
            && key_event.code == KeyCode::Char('k')
        {
            self.mode = Mode::Search;
            self.search.active = true;
            self.table.active = false;
            return;
        } else if key_event.code == KeyCode::Esc {
            self.exit();
            return;
        }

        match self.mode {
            Mode::Search => {
                self.search.handle_key_event(key_event);
                match key_event.code {
                    KeyCode::Backspace => {
                        self.table.packages = self.pacman.search(&self.search.text);
                        self.table.reset();
                    }
                    KeyCode::Char(_) => {
                        self.table.packages = self.pacman.search(&self.search.text);
                        self.table.reset();
                    }
                    _ => {}
                }
            }
            Mode::Table => {
                self.table.handle_key_event(key_event);
                match key_event.code {
                    KeyCode::Char('i') => {
                        stdout().execute(LeaveAlternateScreen).unwrap();
                        disable_raw_mode().unwrap();
                        pacman::install(
                            &self
                                .table
                                .packages
                                .get(self.table.state.selected().unwrap())
                                .unwrap()
                                .name,
                        );
                        stdout().execute(EnterAlternateScreen).unwrap();
                        enable_raw_mode().unwrap();
                        terminal.clear().unwrap();
                    }
                    _ => {}
                }
            }
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
