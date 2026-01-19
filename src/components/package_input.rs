use color_eyre::eyre;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::Component;
use crate::focus::{handle_focus_keys, FocusPosition};
use crate::layout::{INPUT_HEIGHT, LEFT_PANEL_PERCENT};
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
        handle_focus_keys(key_event, &mut self.active, FocusPosition::Top);

        if !self.active {
            return Ok(None);
        }

        let mut actions = Vec::new();

        match *key_event {
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Char(char),
                ..
            } => {
                self.text.push(char);
                actions.push(Action::SearchPackage(self.text.clone()));
            }
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Backspace,
                ..
            } => {
                self.text.pop();
                actions.push(Action::SearchPackage(self.text.clone()));
            }
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('w'),
                ..
            } => {
                if let Some((prefix, _)) = self.text.rsplit_once(' ') {
                    self.text = prefix.to_string();
                } else {
                    self.text.clear();
                }
                actions.push(Action::SearchPackage(self.text.clone()));
            }
            _ => {}
        }

        Ok(Some(actions))
    }

    fn draw(&mut self, frame: &mut Frame, area: &Rect) -> eyre::Result<()> {
        let horizontal_layout = Layout::horizontal([
            Constraint::Percentage(LEFT_PANEL_PERCENT),
            Constraint::Percentage(100 - LEFT_PANEL_PERCENT),
        ])
        .split(*area)[0];
        let area = Layout::vertical([Constraint::Length(INPUT_HEIGHT), Constraint::Percentage(100)])
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
