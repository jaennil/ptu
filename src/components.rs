pub(crate) mod home;
pub(crate) mod search;
pub(crate) mod table;

use std::io;

use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::action::Action;

pub trait Component {
    fn handle_key_event(&mut self, key: KeyEvent) -> io::Result<Vec<Action>> {
        let _ = key; // to appease clippy
        Ok(Vec::new())
    }

    fn update(&mut self, action: &Action) {
        let _ = action;
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()> {
        let _ = frame;
        let _ = area;
        Ok(())
    }
}
