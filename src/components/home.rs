use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    Frame,
};

use std::io;

use crate::{action::Action, components::table::PackagesTable};

use crate::components::{search::PackageSearch, Component};

#[derive(Default)]
pub struct HomeComponent {
    search: PackageSearch,
    table: PackagesTable,
    focus: Focus,
}

#[derive(Default, PartialEq)]
enum Focus {
    #[default]
    Search,
    Table,
}

impl HomeComponent {
    fn set_focus(&mut self, item: Focus) {
        if self.focus == item {
            return;
        }

        match item {
            Focus::Search => {
                self.search.active = true;
                self.table.active = false;
            }
            Focus::Table => {
                self.table.active = true;
                self.search.active = false;
            }
        }

        self.focus = item;
    }

    fn toggle_focus(&mut self) {
        match self.focus {
            Focus::Search => self.set_focus(Focus::Table),
            Focus::Table => self.set_focus(Focus::Search),
        }
    }
}

impl Component for HomeComponent {
    fn handle_key_event(&mut self, key: KeyEvent) -> io::Result<Vec<Option<Action>>> {
        let mut actions = Vec::new();

        match key {
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code,
                ..
            } => match code {
                KeyCode::Char('j') => self.set_focus(Focus::Table),
                KeyCode::Char('k') => self.set_focus(Focus::Search),
                _ => {}
            },

            KeyEvent { code, .. } => match code {
                KeyCode::Tab => self.toggle_focus(),
                _ => {}
            },
        }

        let mut search_actions = self.search.handle_key_event(key)?;
        let mut table_actions = self.table.handle_key_event(key)?;

        actions.append(&mut search_actions);
        actions.append(&mut table_actions);

        Ok(actions)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()> {
        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Percentage(100)]).split(area);
        self.search.draw(frame, layout[0])?;
        self.table.draw(frame, layout[1])?;
        Ok(())
    }
}
