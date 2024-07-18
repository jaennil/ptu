use crate::pacman::Package;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize as _},
    widgets::{Block, Row, Table, TableState},
    Frame,
};

use crate::theme::Theme;

pub struct PackagesTable {
    // TODO
    // widget: Table<'a>,
    pub state: TableState,
    pub packages: Vec<Package>,
    theme: Theme,
    pub active: bool,
}

impl Default for PackagesTable {
    fn default() -> Self {
        Self {
            state: TableState::default().with_selected(Some(0)),
            packages: Vec::default(),
            theme: Theme::default(),
            active: false,
        }
    }
}

impl PackagesTable {
    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.packages.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.packages.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn reset(&mut self) {
        self.state.select(Some(0));
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char('j') => self.next(),
            KeyCode::Char('k') => self.previous(),
            _ => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let mut rows = Vec::new();
        for package in &self.packages {
            rows.push(Row::new(vec![
                package.name.clone(),
                package.description.clone().unwrap_or("".to_string()),
            ]));
        }
        let widths = [Constraint::Percentage(25), Constraint::Percentage(65)];
        let header =
            Row::new(["name", "description"]).style(Style::new().bold().fg(Color::Magenta));
        let border_color = if self.active {
            self.theme.active
        } else {
            self.theme.inactive
        };
        let output = Table::new(rows, widths)
            .header(header)
            .block(Block::bordered().border_style(Style::default().fg(border_color)))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(output, area, &mut self.state);
    }
}
