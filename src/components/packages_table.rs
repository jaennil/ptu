use color_eyre::eyre;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::action::Action;
use crate::components::Component;
use crate::event::Event;
use crate::focus::{handle_focus_keys, FocusPosition};
use crate::layout::{INPUT_HEIGHT, LEFT_PANEL_PERCENT};
use crate::{pacman::Package, theme::Theme};

const COLOR_INSTALLED: Color = Color::Rgb(0, 255, 0);

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

    fn get_selected_package(&mut self) -> Option<&mut Package> {
        let index = self.state.selected()?;
        self.packages.get_mut(index)
    }

    fn reset_selection(&mut self) {
        self.state.select(Some(0));
    }
}

impl Component for PackagesTable {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        handle_focus_keys(key_event, &mut self.active, FocusPosition::Bottom);

        if !self.active {
            return Ok(None);
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
                    if let Some(package) = self.get_selected_package() {
                        actions.push(Action::SelectPackage(Box::new(package.clone())));
                    }
                }
                KeyCode::Char('k') => {
                    self.previous();
                    if let Some(package) = self.get_selected_package() {
                        actions.push(Action::SelectPackage(Box::new(package.clone())));
                    }
                }
                KeyCode::Char('g') => {
                    self.state.select(Some(0));
                    if let Some(package) = self.get_selected_package() {
                        actions.push(Action::SelectPackage(Box::new(package.clone())));
                    }
                }
                KeyCode::Char('i') => {
                    if let Some(package) = self.get_selected_package() {
                        actions.push(Action::InstallPackage {
                            name: package.name.clone(),
                            source: package.source.clone(),
                        });
                    }
                }
                KeyCode::Char('r') => {
                    if let Some(package) = self.get_selected_package() {
                        actions.push(Action::RemovePackage {
                            name: package.name.clone(),
                            source: package.source.clone(),
                        });
                    }
                }
                _ => {}
            },
            KeyEvent {
                modifiers: KeyModifiers::SHIFT,
                code,
                ..
            } => match code {
                KeyCode::Char('G') => {
                    let packages_amount = self.packages.len();
                    self.state.select(Some(packages_amount - 1));
                    if let Some(package) = self.get_selected_package() {
                        actions.push(Action::SelectPackage(Box::new(package.clone())));
                    }
                }
                KeyCode::Char('I') => {
                    if let Some(package) = self.get_selected_package() {
                        actions.push(Action::UpdateInstallPackage {
                            name: package.name.clone(),
                            source: package.source.clone(),
                        });
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
            Event::AurPackagesFound(aur_packages) => {
                // Append AUR packages without resetting selection
                tracing::debug!(count = aur_packages.len(), "merging AUR packages into list");
                self.packages.extend(aur_packages.clone());
            }
            Event::PackageInstalled(package_name) => {
                if let Some(index) = self.packages.iter().position(|p| p.name == *package_name) {
                    self.packages[index].installed = true;
                }
            }
            Event::PackageRemoved(package_name) => {
                if let Some(index) = self.packages.iter().position(|p| p.name == *package_name) {
                    self.packages[index].installed = false;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, area: &Rect) -> eyre::Result<()> {
        let horizontal_layout = Layout::horizontal([
            Constraint::Percentage(LEFT_PANEL_PERCENT),
            Constraint::Percentage(100 - LEFT_PANEL_PERCENT),
        ])
        .split(*area)[0];
        let area = Layout::vertical([Constraint::Length(INPUT_HEIGHT), Constraint::Percentage(100)])
            .split(horizontal_layout)[1];
        let mut rows = Vec::new();
        for package in &self.packages {
            if package.installed {
                let installed = vec![
                    Span::from("["),
                    Span::styled("✔", Style::default().fg(COLOR_INSTALLED)),
                    Span::from("]"),
                ];
                rows.push(Row::new(vec![
                    Cell::from(package.name.clone()),
                    Cell::from(package.source.clone()),
                    Cell::from(Line::from(installed)),
                ]));
            } else {
                rows.push(Row::new(vec![
                    package.name.clone(),
                    package.source.clone(),
                    "[ ]".to_string(),
                ]));
            }
        }
        let widths = [
            Constraint::Fill(1),    // name - takes remaining space
            Constraint::Length(10), // source - fixed
            Constraint::Length(9),  // installed - header width
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
            .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(output, area, &mut self.state);
        Ok(())
    }
}
