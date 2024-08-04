use color_eyre::eyre;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize as _};
use ratatui::widgets::{Block, Row, Table, TableState};
use ratatui::Frame;

use crate::action::Action;
use crate::components::Component;
use crate::event::Event;
use crate::{pacman::Package, theme::Theme};

#[derive(Default)]
pub(crate) struct PackagesTable {
    state: TableState,
    packages: Vec<Package>,
    theme: Theme,
    active: bool,
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

    fn get_selected_package(&self) -> Option<&Package> {
        let index = self.state.selected()?;
        self.packages.get(index)
    }

    fn reset_selection(&mut self) {
        self.state.select(Some(0));
    }
}

impl Component for PackagesTable {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        let actions = None;

        match key_event {
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code,
                ..
            } => match code {
                KeyCode::Char('j') => self.active = true,
                KeyCode::Char('k') => self.active = false,
                _ => {}
            },
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code,
                ..
            } => match code {
                KeyCode::Tab => self.active = !self.active,
                _ => {}
            },
            _ => {}
        }

        if !self.active {
            return Ok(actions);
        }

        let mut actions = Vec::new();

        match key_event {
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code,
                ..
            } => match code {
                KeyCode::Char('j') => {
                    self.next();
                    let package = self.get_selected_package();
                    if let Some(package) = package {
                        actions.push(Action::SelectPackage(package.clone()));
                    }
                }
                KeyCode::Char('k') => {
                    self.previous();
                    let package = self.get_selected_package();
                    if let Some(package) = package {
                        actions.push(Action::SelectPackage(package.clone()));
                    }
                }
                KeyCode::Char('i') => {
                    if let Some(package) = self.get_selected_package() {
                        let package_name = package.name.to_string();
                        actions.push(Action::InstallPackage(package_name));
                    }
                }
                _ => {}
            },
            _ => {}
        }

        Ok(Some(actions))
    }

    fn update(&mut self, event: &Event) -> eyre::Result<()> {
        match event {
            Event::FoundPackages(packages) => {
                self.packages = packages.clone();
                self.reset_selection();
            }
            _ => {}
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, area: &Rect) -> eyre::Result<()> {
        let horizontal_layout =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(*area)[0];
        let area = Layout::vertical([Constraint::Length(3), Constraint::Percentage(100)])
            .split(horizontal_layout)[1];
        let mut rows = Vec::new();
        for package in &self.packages {
            rows.push(Row::new(vec![
                package.name.clone(),
                package.source.clone(),
                package.installed.to_string(),
            ]));
        }
        let widths = [
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Percentage(100),
        ];
        let header =
            Row::new(["name", "source", "installed"]).style(Style::new().bold().fg(Color::Magenta));
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
