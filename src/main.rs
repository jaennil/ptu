use std::io::{self};

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Paragraph, Row, Table, TableState},
    Frame,
};

mod pacman;
mod tui;

use pacman::{Package, Pacman};

fn main() -> io::Result<()> {
    let mut terminal = tui::init()?;
    let mut app = App::new();
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
    search: String,
    packages: Vec<Package>,
    table_state: TableState,
    mode: Mode,
    exit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            table_state: TableState::default().with_selected(0),
            ..Default::default()
        }
    }

    fn run(&mut self, terminal: &mut tui::Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(frame.size());

        let search = Paragraph::new(self.search.clone()).block(Block::bordered());
        frame.render_widget(search, layout[0]);

        let mut rows = Vec::new();
        for package in &self.packages {
            rows.push(Row::new(vec![
                package.name.clone(),
                package.description.clone().unwrap_or("".to_string()),
            ]));
        }
        let widths = [Constraint::Percentage(25), Constraint::Percentage(65)];
        let header = Row::new(["name", "description"])
            .style(Style::new().bold().fg(Color::Magenta))
            .bottom_margin(1);
        let output = Table::new(rows, widths)
            .header(header)
            .block(Block::bordered())
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">>");
        frame.render_stateful_widget(output, layout[1], &mut self.table_state);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('j') {
            self.mode = Mode::Table;
        } else if key_event.modifiers == KeyModifiers::CONTROL
            && key_event.code == KeyCode::Char('j')
        {
            self.mode = Mode::Table;
        } else if key_event.modifiers == KeyModifiers::CONTROL
            && key_event.code == KeyCode::Char('k')
        {
            self.mode = Mode::Search;
        } else if key_event.code == KeyCode::Esc {
            self.exit();
        }

        match self.mode {
            Mode::Search => match key_event.code {
                KeyCode::Backspace => {
                    self.search.pop();
                    self.packages = self.pacman.search(&self.search);
                }
                // TODO: fix control
                KeyCode::Char(value) => {
                    self.search.push(value);
                    self.packages = self.pacman.search(&self.search);
                }
                _ => {}
            },
            Mode::Table => match key_event.code {
                KeyCode::Char('j') => self.next(),
                KeyCode::Char('k') => self.previous(),
                _ => {}
            },
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.packages.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.packages.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        // self.scroll_table_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }
}
