pub(crate) mod home;
mod search;
mod table;

use std::io;

use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::action::Action;

pub trait Component {
    fn handle_key_event(&mut self, key: KeyEvent) -> io::Result<Vec<Option<Action>>> {
        let _ = key; // to appease clippy
        Ok(Vec::new())
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()>;
}
