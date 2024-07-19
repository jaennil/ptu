use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    Frame,
};

use std::io;

use crate::table::PackagesTable;

use crate::{component::Component, search::PackageSearch};

#[derive(Default)]
pub struct HomeComponent {
    search: PackageSearch,
    table: PackagesTable,
}

impl Component for HomeComponent {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()> {
        let layout =
            Layout::vertical([Constraint::Length(3), Constraint::Percentage(100)]).split(area);
        self.search.draw(frame, layout[0])?;
        self.table.draw(frame, layout[1])?;
        // let search = Paragraph::new(self.text.clone())
        //     .block(Block::bordered().border_style(Style::default()));
        // frame.render_widget(search, area);
        Ok(())
    }
    fn handle_key_event(&mut self, key: KeyEvent) {
        match key {
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code,
                ..
            } => match code {
                KeyCode::Char('j') => {
                    self.table.active = true;
                    self.search.active = false;
                }
                KeyCode::Char('k') => {
                    self.search.active = true;
                    self.table.active = false
                }
                _ => {}
            },
            _ => {}
        }
        self.search.handle_key_event(key);
        self.table.handle_key_event(key);
    }
}
