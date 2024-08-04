pub(crate) mod package_input;
pub(crate) mod packages_table;

use crate::action::Action;
use crate::event::Event;

use color_eyre::eyre;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub(crate) trait Component {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        let _ = key_event;
        Ok(None)
    }

    fn update(&mut self, event: &Event) -> eyre::Result<()> {
        let _ = event;
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, area: &Rect) -> eyre::Result<()> {
        let _ = frame;
        let _ = area;
        Ok(())
    }
}
