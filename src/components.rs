pub(crate) mod installed_table;
pub(crate) mod package_info;
pub(crate) mod package_input;
pub(crate) mod packages_table;

use crate::action::Action;
use crate::event::Event;

use color_eyre::eyre;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(crate) trait Component {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        let _ = key_event;
        Ok(None)
    }

    fn handle_mouse_event(
        &mut self,
        mouse_event: &MouseEvent,
        area: &Rect,
    ) -> eyre::Result<Option<Vec<Action>>> {
        let _ = mouse_event;
        let _ = area;
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

    fn set_active(&mut self, active: bool) {
        let _ = active;
    }

    fn is_name_filter_mode(&self) -> bool {
        false
    }
}
