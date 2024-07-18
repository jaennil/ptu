use ratatui::style::Color;

pub struct Theme {
    pub active: Color,
    pub inactive: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            active: Color::White,
            inactive: Color::DarkGray,
        }
    }
}
