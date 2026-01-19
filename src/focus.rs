use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(crate) enum FocusPosition {
    Top,
    Bottom,
}

pub(crate) fn handle_focus_keys(key_event: &KeyEvent, active: &mut bool, position: FocusPosition) -> bool {
    match key_event {
        KeyEvent {
            modifiers: KeyModifiers::CONTROL,
            code,
            ..
        } => match (code, &position) {
            (KeyCode::Char('k'), FocusPosition::Top) => {
                *active = true;
                true
            }
            (KeyCode::Char('j'), FocusPosition::Top) => {
                *active = false;
                true
            }
            (KeyCode::Char('j'), FocusPosition::Bottom) => {
                *active = true;
                true
            }
            (KeyCode::Char('k'), FocusPosition::Bottom) => {
                *active = false;
                true
            }
            _ => false,
        },
        KeyEvent {
            modifiers: KeyModifiers::NONE,
            code: KeyCode::Tab,
            ..
        } => {
            *active = !*active;
            true
        }
        _ => false,
    }
}
