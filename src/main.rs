use std::{io::{self}, process::Command};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Paragraph, Widget},
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
    output: String,
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
            KeyCode::Char(value) => {
                self.search.push(value);
                let output = Command::new("pacman")
                    .arg("-Ss")
                    .arg(&self.search)
                    .output()
                    .expect("failed to execute process");
                let s = std::str::from_utf8(&output.stdout).unwrap().to_string();
                self.output = s;
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

        let output = Paragraph::new(self.output.clone()).block(Block::bordered());
        output.render(layout[1], buf);
    }
}
