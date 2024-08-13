use color_eyre::eyre;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::Component;
use crate::theme::Theme;

pub(crate) struct PackageInput {
    text: String,
    theme: Theme,
    active: bool,
}

impl Default for PackageInput {
    fn default() -> Self {
        Self {
            text: Default::default(),
            theme: Default::default(),
            active: true,
        }
    }
}

impl Component for PackageInput {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        let actions = None;

        match *key_event {
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code,
                ..
            } => match code {
                KeyCode::Char('k') => self.active = true,
                KeyCode::Char('j') => self.active = false,
                _ => {}
            },
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code,
                ..
            } => match code {
                KeyCode::Tab => self.active = !self.active,
                _ => {}
            },
            _ => {}
        }

        if !self.active {
            return Ok(actions);
        }

        let mut actions = Vec::new();

        match *key_event {
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code,
                ..
            } => match code {
                KeyCode::Char(char) => {
                    self.text.push(char);
                    actions.push(Action::SearchPackage(self.text.clone()));
                }
                KeyCode::Backspace => {
                    self.text.pop();
                    actions.push(Action::SearchPackage(self.text.clone()));
                }
                _ => {}
            },
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code,
                ..
            } => match code {
                KeyCode::Char('w') => {
                    let without_last_word = self.text.rsplit_once(' ');
                    if let Some(parts) = without_last_word {
                        self.text = parts.0.to_string();
                    } else {
                        self.text = String::from("");
                    }
                    actions.push(Action::SearchPackage(self.text.clone()));
                }
                _ => {}
            },
            _ => {}
        }

        Ok(Some(actions))
    }

    fn draw(&mut self, frame: &mut Frame, area: &Rect) -> eyre::Result<()> {
        let horizontal_layout =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(*area)[0];
        let area = Layout::vertical([Constraint::Length(3), Constraint::Percentage(100)])
            .split(horizontal_layout)[0];
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
}
