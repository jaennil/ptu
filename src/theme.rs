use ratatui::style::Color;

pub(crate) struct Theme {
    pub(crate) active: Color,
    pub(crate) inactive: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            active: Color::White,
            inactive: Color::DarkGray,
        }
    }
}
