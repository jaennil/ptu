use std::{
    io::{self},
    process::Command,
};

use alpm::{Alpm, SigLevel};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Paragraph, Row, Table, Widget},
    Frame,
};

mod tui;

fn main() -> io::Result<()> {
    let mut terminal = tui::init()?;
    let mut app = App::default();
    let result = app.run(&mut terminal);
    tui::restore()?;
    result
}

#[derive(Default)]
struct App {
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
                self.packages = search(&self.search);
            }
            KeyCode::Char(value) => {
                self.search.push(value);
                self.packages = search(&self.search);
            }
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

struct Package {
    name: String,
    description: Option<String>,
}

fn search(package: &str) -> Vec<Package> {
    let handle = Alpm::new("/", "/var/lib/pacman").unwrap();

    handle
        .register_syncdb("core", SigLevel::USE_DEFAULT)
        .unwrap();
    handle
        .register_syncdb("extra", SigLevel::USE_DEFAULT)
        .unwrap();
    handle
        .register_syncdb("community", SigLevel::USE_DEFAULT)
        .unwrap();

    let mut res = Vec::new();
    for db in handle.syncdbs() {
        for pkg in db.search([package].iter()).unwrap() {
            res.push(Package {
                name: pkg.name().to_string(),
                description: pkg.desc().map(str::to_string),
            });
        }
    }

    res
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
