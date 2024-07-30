use std::io::{self};

use crate::{action::Action, components::Component, pacman::Package};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize as _},
    widgets::{Block, Row, Table, TableState},
    Frame,
};

use crate::theme::Theme;

use super::home::Focus;

pub struct PackagesTable {
    // TODO:
    // widget: Table<'a>,
    pub state: TableState,
    pub packages: Vec<Package>,
    theme: Theme,
    pub active: bool,
    // TODO: dont store package here instead handle it through search event
}

impl Default for PackagesTable {
    fn default() -> Self {
        Self {
            // TODO: accept event that table is now active and remove this with selected stuff
            // in the event call select if any packages
            state: TableState::default().with_selected(Some(0)),
            packages: Default::default(),
            theme: Default::default(),
            active: Default::default(),
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

    fn reset_selection(&mut self) {
        self.state.select(Some(0));
    }
}

impl Component for PackagesTable {
    fn handle_key_event(&mut self, event: KeyEvent) -> io::Result<Vec<Action>> {
        let mut actions = Vec::new();

        if !self.active {
            return Ok(actions);
        }

        match event {
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code,
                ..
            } => match code {
                KeyCode::Char('j') => self.next(),
                KeyCode::Char('k') => self.previous(),
                KeyCode::Char('i') => {
                    let i = self.state.selected().unwrap();
                    let package_name = self.packages.get(i).unwrap().name();
                    actions.push(Action::InstallPackage(package_name.to_string()));
                }
                _ => {}
            },
            _ => {}
        }

        Ok(actions)
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::FoundPackages(packages) => {
                self.packages = (*packages).clone();
                self.reset_selection();
            }
            Action::Focus(focus) => {
                if *focus == Focus::Table {
                    self.active = true;
                } else {
                    self.active = false;
                }
            }
            _ => {}
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> io::Result<()> {
        let area =
            Layout::vertical([Constraint::Length(3), Constraint::Percentage(100)]).split(area)[1];
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
        Ok(())
    }
}
