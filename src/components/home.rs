use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::io;

use crate::action::Action;

use crate::components::Component;

#[derive(Default)]
pub struct HomeComponent {
    focus: Focus,
}

#[derive(Default, PartialEq, Clone)]
pub enum Focus {
    #[default]
    Search,
    Table,
}

impl From<char> for Focus {
    fn from(value: char) -> Self {
        match value {
            'j' => Focus::Table,
            'k' => Focus::Search,
            _ => panic!("should be impossible to get char rather than `j` and `k`"),
        }
    }
}

impl HomeComponent {
    fn set_focus(&mut self, item: Focus) -> Option<Action> {
        let mut action = None;

        if self.focus == item {
            return action;
        }

        match item {
            Focus::Search => {
                action = Some(Action::Focus(Focus::Search));
            }
            Focus::Table => {
                action = Some(Action::Focus(Focus::Table));
            }
        }

        self.focus = item;

        action
    }

    fn toggle_focus(&mut self) -> Action {
        match self.focus {
            Focus::Search => self
                .set_focus(Focus::Table)
                .expect("should be impossible to get None here"),
            Focus::Table => self
                .set_focus(Focus::Search)
                .expect("should be impossible to get None here"),
        }
    }
}

impl Component for HomeComponent {
    fn handle_key_event(&mut self, key: &KeyEvent) -> io::Result<Vec<Action>> {
        let mut actions = Vec::new();

        match key {
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code,
                ..
            } => match code {
                KeyCode::Char(jk @ ('j' | 'k')) => {
                    let action = self.set_focus(Focus::from(*jk));
                    if action.is_some() {
                        actions.push(action.unwrap());
                    }
                }
                _ => {}
            },

            KeyEvent { code, .. } => match code {
                KeyCode::Tab => {
                    actions.push(self.toggle_focus());
                }
                _ => {}
            },
        }

        Ok(actions)
    }
}
