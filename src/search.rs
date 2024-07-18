use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::Rect,
    style::Style,
    widgets::{Block, Paragraph},
    Frame,
};

use crate::theme::Theme;

pub struct PackageSearch {
    pub text: String,
    theme: Theme,
    pub active: bool,
}

impl Default for PackageSearch {
    fn default() -> Self {
        Self {
            text: Default::default(),
            theme: Default::default(),
            active: true,
        }
    }
}

impl PackageSearch {
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let border_color = if self.active {
            self.theme.active
        } else {
            self.theme.inactive
        };
        let search = Paragraph::new(self.text.clone())
            .block(Block::bordered().border_style(Style::default().fg(border_color)));
        frame.render_widget(search, area);
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Backspace => {
                self.text.pop();
            }
            KeyCode::Char(value) => {
                self.text.push(value);
            }
            _ => {}
        }
    }
}
