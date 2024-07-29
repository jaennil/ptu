use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    style::Style,
    widgets::{Block, Paragraph},
    Frame,
};

use std::io;

use crate::{action::Action, components::Component};

use crate::theme::Theme;

pub struct PackageSearch {
    text: String,
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

impl Component for PackageSearch {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()> {
        let border_color = if self.active {
            self.theme.active
        } else {
            self.theme.inactive
        };
        let search = Paragraph::new(self.text.clone())
            .block(Block::bordered().border_style(Style::default().fg(border_color)));
        frame.render_widget(search, area);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> io::Result<Vec<Action>> {
        let mut actions = Vec::new();

        if !self.active {
            return Ok(actions);
        }

        match key {
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code,
                ..
            } => {
                match code {
                    KeyCode::Char(char) => {
                        self.text.push(char);
                    }
                    KeyCode::Backspace => {
                        self.text.pop();
                    }
                    _ => {}
                };
                actions.push(Action::SearchPackage(self.text.clone()));
            }
            _ => {}
        };
        Ok(actions)
    }
}
