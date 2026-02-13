use std::collections::HashSet;

use color_eyre::eyre;
use ratatui::crossterm::event::{KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::action::Action;
use crate::components::Component;
use crate::config::{self, PackagesTableKeys};
use crate::event::Event;
use crate::filter::{InstallFilter, PackageFilter, SourceFilter};
use crate::layout::{FILTER_HEIGHT, INPUT_HEIGHT, LEFT_PANEL_PERCENT};
use crate::{pacman::Package, theme::Theme};

const COLOR_INSTALLED: Color = Color::Rgb(0, 255, 0);
const COLOR_SELECTED: Color = Color::Rgb(255, 165, 0); // Orange
const COLOR_FILTER_ACTIVE: Color = Color::Rgb(0, 255, 127); // Bright green
const COLOR_FILTER_INACTIVE: Color = Color::Rgb(128, 128, 128); // Dim gray

pub(crate) struct PackagesTable {
    state: TableState,
    all_packages: Vec<Package>,
    theme: Theme,
    active: bool,
    selected_indices: HashSet<usize>,
    filter: PackageFilter,
    filter_mode: bool,
    keys: PackagesTableKeys,
}

impl PackagesTable {
    pub(crate) fn new(keys: PackagesTableKeys) -> Self {
        Self {
            state: TableState::default(),
            all_packages: Vec::new(),
            theme: Theme::default(),
            active: false,
            selected_indices: HashSet::new(),
            filter: PackageFilter::default(),
            filter_mode: false,
            keys,
        }
    }

    /// Returns packages that match the current filter
    fn filtered_packages(&self) -> Vec<(usize, &Package)> {
        self.all_packages
            .iter()
            .enumerate()
            .filter(|(_, pkg)| self.filter.matches(pkg))
            .collect()
    }

    fn next(&mut self) {
        let filtered = self.filtered_packages();
        if filtered.is_empty() {
            return;
        }

        let current_idx = self.state.selected().unwrap_or(0);
        // Find position of current index in filtered list
        let current_pos = filtered.iter().position(|(idx, _)| *idx == current_idx);

        let next_pos = match current_pos {
            Some(pos) => {
                if pos >= filtered.len() - 1 {
                    0
                } else {
                    pos + 1
                }
            }
            None => 0,
        };

        self.state.select(Some(filtered[next_pos].0));
    }

    fn previous(&mut self) {
        let filtered = self.filtered_packages();
        if filtered.is_empty() {
            return;
        }

        let current_idx = self.state.selected().unwrap_or(0);
        // Find position of current index in filtered list
        let current_pos = filtered.iter().position(|(idx, _)| *idx == current_idx);

        let prev_pos = match current_pos {
            Some(pos) => {
                if pos == 0 {
                    filtered.len() - 1
                } else {
                    pos - 1
                }
            }
            None => 0,
        };

        self.state.select(Some(filtered[prev_pos].0));
    }

    fn get_selected_package(&mut self) -> Option<&mut Package> {
        let index = self.state.selected()?;
        self.all_packages.get_mut(index)
    }

    fn reset_selection(&mut self) {
        // Find first filtered package index
        let first_filtered_idx = self
            .all_packages
            .iter()
            .enumerate()
            .find(|(_, pkg)| self.filter.matches(pkg))
            .map(|(idx, _)| idx);

        if let Some(idx) = first_filtered_idx {
            self.state.select(Some(idx));
            tracing::debug!(index = idx, "selection reset to first filtered package");
        } else {
            self.state.select(None);
            tracing::debug!("no packages match filter, selection cleared");
        }
    }

    fn toggle_selection(&mut self) {
        if let Some(index) = self.state.selected() {
            if self.selected_indices.contains(&index) {
                tracing::debug!(index, "deselecting package");
                self.selected_indices.remove(&index);
            } else {
                tracing::debug!(index, "selecting package");
                self.selected_indices.insert(index);
            }
        }
    }

    fn clear_selection(&mut self) {
        if !self.selected_indices.is_empty() {
            tracing::debug!(count = self.selected_indices.len(), "clearing selection");
            self.selected_indices.clear();
        }
    }

    fn get_selected_packages(&self) -> Vec<(String, String)> {
        self.selected_indices
            .iter()
            .filter_map(|&idx| {
                self.all_packages.get(idx).map(|p| (p.name.clone(), p.source.clone()))
            })
            .collect()
    }

    fn go_to_first(&mut self) {
        let filtered = self.filtered_packages();
        if let Some((first_idx, _)) = filtered.first() {
            self.state.select(Some(*first_idx));
        }
    }

    fn go_to_last(&mut self) {
        let filtered = self.filtered_packages();
        if let Some((last_idx, _)) = filtered.last() {
            self.state.select(Some(*last_idx));
        }
    }

    fn draw_filter_bar(&self, frame: &mut Frame, area: Rect) {
        // Build filter bar line
        let mut spans = Vec::new();

        // Filter mode indicator
        if self.filter_mode {
            spans.push(Span::styled(
                "[FILTER] ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
        }

        // Install filter
        let install_color = if self.filter.install != InstallFilter::All {
            COLOR_FILTER_ACTIVE
        } else {
            COLOR_FILTER_INACTIVE
        };
        spans.push(Span::styled(
            format!("[i:{}]", self.filter.install.label()),
            Style::default().fg(install_color),
        ));

        spans.push(Span::from(" "));

        // Source filter
        let source_color = if self.filter.source != SourceFilter::All {
            COLOR_FILTER_ACTIVE
        } else {
            COLOR_FILTER_INACTIVE
        };
        spans.push(Span::styled(
            format!("[s:{}]", self.filter.source.label()),
            Style::default().fg(source_color),
        ));

        // Package statistics
        if !self.all_packages.is_empty() {
            let total = self.all_packages.len();
            let aur_count = self.all_packages.iter().filter(|p| p.source == "aur").count();
            let pacman_count = total - aur_count;
            let filtered_count = self.filtered_packages().len();

            spans.push(Span::from(" "));

            if filtered_count != total {
                spans.push(Span::styled(
                    format!("{}/{}", filtered_count, total),
                    Style::default().fg(Color::DarkGray),
                ));
            } else {
                spans.push(Span::styled(
                    format!("{}", total),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            spans.push(Span::styled(
                format!(" (repo:{} aur:{})", pacman_count, aur_count),
                Style::default().fg(Color::DarkGray),
            ));
        }

        let line = Line::from(spans);
        frame.render_widget(line, area);
    }
}

impl Component for PackagesTable {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        if !self.active {
            return Ok(None);
        }

        let mut actions = Vec::new();

        // Handle filter mode keys
        if self.filter_mode {
            if config::key_matches(key_event, &self.keys.filter.cycle_install) {
                tracing::debug!("filter mode: cycling install filter");
                self.filter.install.cycle();
                self.reset_selection();
                self.clear_selection();
                self.filter_mode = false;
                if let Some(package) = self.get_selected_package() {
                    actions.push(Action::SelectPackage(Box::new(package.clone())));
                }
            } else if config::key_matches(key_event, &self.keys.filter.toggle_aur) {
                tracing::debug!("filter mode: toggling AUR filter");
                self.filter.source = if self.filter.source == SourceFilter::Aur {
                    SourceFilter::All
                } else {
                    SourceFilter::Aur
                };
                self.reset_selection();
                self.clear_selection();
                self.filter_mode = false;
                if let Some(package) = self.get_selected_package() {
                    actions.push(Action::SelectPackage(Box::new(package.clone())));
                }
            } else if config::key_matches(key_event, &self.keys.filter.toggle_pacman) {
                tracing::debug!("filter mode: toggling Pacman filter");
                self.filter.source = if self.filter.source == SourceFilter::Pacman {
                    SourceFilter::All
                } else {
                    SourceFilter::Pacman
                };
                self.reset_selection();
                self.clear_selection();
                self.filter_mode = false;
                if let Some(package) = self.get_selected_package() {
                    actions.push(Action::SelectPackage(Box::new(package.clone())));
                }
            } else if config::key_matches(key_event, &self.keys.filter.clear_all) {
                tracing::debug!("filter mode: clearing all filters");
                self.filter.clear();
                self.reset_selection();
                self.clear_selection();
                self.filter_mode = false;
                if let Some(package) = self.get_selected_package() {
                    actions.push(Action::SelectPackage(Box::new(package.clone())));
                }
            } else if config::key_matches(key_event, &self.keys.filter.exit) {
                tracing::debug!("filter mode: exiting");
                self.filter_mode = false;
            } else {
                // Any other key exits filter mode without action
                self.filter_mode = false;
            }
            return Ok(Some(actions));
        }

        // Normal mode key handling
        if config::key_matches(key_event, &self.keys.filter_mode) {
            tracing::debug!("entering filter mode");
            self.filter_mode = true;
        } else if config::key_matches(key_event, &self.keys.next) {
            self.next();
            if let Some(package) = self.get_selected_package() {
                actions.push(Action::SelectPackage(Box::new(package.clone())));
            }
        } else if config::key_matches(key_event, &self.keys.previous) {
            self.previous();
            if let Some(package) = self.get_selected_package() {
                actions.push(Action::SelectPackage(Box::new(package.clone())));
            }
        } else if config::key_matches(key_event, &self.keys.first) {
            self.go_to_first();
            if let Some(package) = self.get_selected_package() {
                actions.push(Action::SelectPackage(Box::new(package.clone())));
            }
        } else if config::key_matches(key_event, &self.keys.last) {
            self.go_to_last();
            if let Some(package) = self.get_selected_package() {
                actions.push(Action::SelectPackage(Box::new(package.clone())));
            }
        } else if config::key_matches(key_event, &self.keys.install) {
            if let Some(package) = self.get_selected_package() {
                actions.push(Action::InstallPackage {
                    name: package.name.clone(),
                    source: package.source.clone(),
                });
            }
        } else if config::key_matches(key_event, &self.keys.remove) {
            if let Some(package) = self.get_selected_package() {
                actions.push(Action::RemovePackage {
                    name: package.name.clone(),
                    source: package.source.clone(),
                });
            }
        } else if config::key_matches(key_event, &self.keys.batch_install) {
            let selected = self.get_selected_packages();
            if !selected.is_empty() {
                tracing::info!(count = selected.len(), "batch install selected packages");
                actions.push(Action::InstallPackages { packages: selected });
            } else if let Some(package) = self.get_selected_package() {
                // Fallback to single update+install when no selection
                actions.push(Action::UpdateInstallPackage {
                    name: package.name.clone(),
                    source: package.source.clone(),
                });
            }
        } else if config::key_matches(key_event, &self.keys.batch_remove) {
            let selected = self.get_selected_packages();
            if !selected.is_empty() {
                tracing::info!(count = selected.len(), "batch remove selected packages");
                actions.push(Action::RemovePackages { packages: selected });
            } else if let Some(package) = self.get_selected_package() {
                // Fallback to single remove when no selection
                actions.push(Action::RemovePackage {
                    name: package.name.clone(),
                    source: package.source.clone(),
                });
            }
        } else if config::key_matches(key_event, &self.keys.multi_select) {
            self.toggle_selection();
        }

        Ok(Some(actions))
    }

    fn update(&mut self, event: &Event) -> eyre::Result<()> {
        match event {
            Event::FoundPackages(packages) => {
                self.all_packages = packages.clone();
                self.reset_selection();
                self.clear_selection(); // Clear selection on new search (indices invalidate)
            }
            Event::AurPackagesFound(aur_packages) => {
                // Append AUR packages without resetting selection
                tracing::debug!(count = aur_packages.len(), "merging AUR packages into list");
                self.all_packages.extend(aur_packages.clone());
            }
            Event::PackageInstalled(package_name) => {
                if let Some(index) = self.all_packages.iter().position(|p| p.name == *package_name) {
                    self.all_packages[index].installed = true;
                }
            }
            Event::PackageRemoved(package_name) => {
                if let Some(index) = self.all_packages.iter().position(|p| p.name == *package_name) {
                    self.all_packages[index].installed = false;
                }
            }
            Event::PackagesInstalled(names) => {
                tracing::debug!(count = names.len(), "marking packages as installed");
                for name in names {
                    if let Some(index) = self.all_packages.iter().position(|p| &p.name == name) {
                        self.all_packages[index].installed = true;
                    }
                }
                self.clear_selection();
            }
            Event::PackagesRemoved(names) => {
                tracing::debug!(count = names.len(), "marking packages as removed");
                for name in names {
                    if let Some(index) = self.all_packages.iter().position(|p| &p.name == name) {
                        self.all_packages[index].installed = false;
                    }
                }
                self.clear_selection();
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

        // Split area for filter bar and table
        let vertical_areas = Layout::vertical([
            Constraint::Length(INPUT_HEIGHT),
            Constraint::Length(FILTER_HEIGHT),
            Constraint::Percentage(100),
        ])
        .split(horizontal_layout);

        let filter_area = vertical_areas[1];
        let table_area = vertical_areas[2];

        // Draw filter bar
        self.draw_filter_bar(frame, filter_area);

        // Get filtered packages
        let filtered = self.filtered_packages();

        let mut rows = Vec::new();
        for (idx, package) in filtered.iter() {
            let is_selected = self.selected_indices.contains(idx);
            let selection_marker = if is_selected {
                Span::styled("*", Style::default().fg(COLOR_SELECTED))
            } else {
                Span::from(" ")
            };

            let status_cell = if package.installed {
                Line::from(vec![
                    selection_marker,
                    Span::from("["),
                    Span::styled("✔", Style::default().fg(COLOR_INSTALLED)),
                    Span::from("]"),
                ])
            } else {
                Line::from(vec![
                    selection_marker,
                    Span::from("[ ]"),
                ])
            };

            rows.push(Row::new(vec![
                Cell::from(package.name.clone()),
                Cell::from(package.source.clone()),
                Cell::from(status_cell),
            ]));
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

        // Map the internal selection to the filtered view row position
        let mut display_state = TableState::default();
        if let Some(selected_idx) = self.state.selected() {
            // Find the position of selected_idx in the filtered list
            if let Some(pos) = filtered.iter().position(|(idx, _)| *idx == selected_idx) {
                display_state.select(Some(pos));
            }
        }

        frame.render_stateful_widget(output, table_area, &mut display_state);
        Ok(())
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn handle_mouse_event(
        &mut self,
        mouse_event: &MouseEvent,
        area: &Rect,
    ) -> eyre::Result<Option<Vec<Action>>> {
        let (x, y) = (mouse_event.column, mouse_event.row);
        if !area.contains((x, y).into()) {
            return Ok(None);
        }

        let mut actions = Vec::new();

        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                tracing::trace!("packages table scroll up");
                self.previous();
                if let Some(package) = self.get_selected_package() {
                    actions.push(Action::SelectPackage(Box::new(package.clone())));
                }
            }
            MouseEventKind::ScrollDown => {
                tracing::trace!("packages table scroll down");
                self.next();
                if let Some(package) = self.get_selected_package() {
                    actions.push(Action::SelectPackage(Box::new(package.clone())));
                }
            }
            _ => {}
        }

        Ok(Some(actions))
    }
}
