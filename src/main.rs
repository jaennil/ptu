use std::io::{self};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Paragraph, Row, Table, Widget},
    Frame,
};

mod pacman;
mod tui;

use pacman::{Package, Pacman};

fn main() -> io::Result<()> {
    let mut terminal = tui::init()?;
    let mut app = App::default();
    let result = app.run(&mut terminal);
    tui::restore()?;
    result
}

#[derive(Default)]
struct App {
    pacman: Pacman,
    search: String,
    packages: Vec<Package>,
    exit: bool,
}

impl App {
    fn run(&mut self, terminal: &mut tui::Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
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
        match key_event.code {
            KeyCode::Esc => self.exit(),
            KeyCode::Backspace => {
                self.search.pop();
                self.packages = self.pacman.search(&self.search);
            }
            KeyCode::Char(value) => {
                self.search.push(value);
                self.packages = self.pacman.search(&self.search);
            }
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(area);

        let search = Paragraph::new(self.search.clone()).block(Block::bordered());
        search.render(layout[0], buf);

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
            .highlight_symbol(">>");
        output.render(layout[1], buf);
    }
}
