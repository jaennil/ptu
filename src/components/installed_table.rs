use std::collections::HashSet;

use color_eyre::eyre;
use ratatui::crossterm::event::{KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::action::Action;
use crate::components::Component;
use crate::config::{self, InstalledTableKeys};
use crate::event::Event;
use crate::pacman::{format_size, format_timestamp, Package};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortColumn {
    Name,
    Size,
    InstallDate,
    DepsCount,
}

impl SortColumn {
    fn label(&self) -> &'static str {
        match self {
            SortColumn::Name => "Name",
            SortColumn::Size => "Size",
            SortColumn::InstallDate => "Date",
            SortColumn::DepsCount => "Deps",
        }
    }

    fn next(&self) -> SortColumn {
        match self {
            SortColumn::Name => SortColumn::Size,
            SortColumn::Size => SortColumn::InstallDate,
            SortColumn::InstallDate => SortColumn::DepsCount,
            SortColumn::DepsCount => SortColumn::Name,
        }
    }
}

pub(crate) struct InstalledTable {
    packages: Vec<Package>,
    sorted_indices: Vec<usize>,
    sort_column: SortColumn,
    ascending: bool,
    state: TableState,
    active: bool,
    theme: Theme,
    selected_indices: HashSet<usize>,
    keys: InstalledTableKeys,
}

impl InstalledTable {
    pub(crate) fn new(packages: Vec<Package>, keys: InstalledTableKeys) -> Self {
        tracing::debug!(count = packages.len(), "creating InstalledTable");
        let mut table = Self {
            packages,
            sorted_indices: Vec::new(),
            sort_column: SortColumn::Name,
            ascending: true,
            state: TableState::default(),
            active: false,
            theme: Theme::default(),
            selected_indices: HashSet::new(),
            keys,
        };
        table.resort();
        if !table.sorted_indices.is_empty() {
            table.state.select(Some(0));
        }
        table
    }

    fn resort(&mut self) {
        self.sorted_indices = (0..self.packages.len()).collect();

        let packages = &self.packages;
        let sort_column = self.sort_column;
        let ascending = self.ascending;

        self.sorted_indices.sort_by(|&a, &b| {
            let cmp = match sort_column {
                SortColumn::Name => packages[a].name.to_lowercase().cmp(&packages[b].name.to_lowercase()),
                SortColumn::Size => packages[a].size.cmp(&packages[b].size),
                SortColumn::InstallDate => {
                    let da = packages[a].install_date.unwrap_or(0);
                    let db = packages[b].install_date.unwrap_or(0);
                    da.cmp(&db)
                }
                SortColumn::DepsCount => packages[a].depends.len().cmp(&packages[b].depends.len()),
            };
            if ascending { cmp } else { cmp.reverse() }
        });

        tracing::debug!(
            column = ?self.sort_column,
            ascending = self.ascending,
            count = self.sorted_indices.len(),
            "resorted installed table"
        );
    }

    fn selected_package_index(&self) -> Option<usize> {
        let display_idx = self.state.selected()?;
        self.sorted_indices.get(display_idx).copied()
    }

    pub(crate) fn get_selected_package(&self) -> Option<&Package> {
        let pkg_idx = self.selected_package_index()?;
        self.packages.get(pkg_idx)
    }

    fn next(&mut self) {
        if self.sorted_indices.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let next = if current >= self.sorted_indices.len() - 1 {
            0
        } else {
            current + 1
        };
        self.state.select(Some(next));
    }

    fn previous(&mut self) {
        if self.sorted_indices.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let prev = if current == 0 {
            self.sorted_indices.len() - 1
        } else {
            current - 1
        };
        self.state.select(Some(prev));
    }

    fn go_to_first(&mut self) {
        if !self.sorted_indices.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn go_to_last(&mut self) {
        if !self.sorted_indices.is_empty() {
            self.state.select(Some(self.sorted_indices.len() - 1));
        }
    }

    fn toggle_selection(&mut self) {
        if let Some(pkg_idx) = self.selected_package_index() {
            if self.selected_indices.contains(&pkg_idx) {
                tracing::debug!(pkg_idx, "deselecting installed package");
                self.selected_indices.remove(&pkg_idx);
            } else {
                tracing::debug!(pkg_idx, "selecting installed package");
                self.selected_indices.insert(pkg_idx);
            }
        }
    }

    fn get_selected_packages(&self) -> Vec<(String, String)> {
        self.selected_indices
            .iter()
            .filter_map(|&idx| {
                self.packages.get(idx).map(|p| (p.name.clone(), p.source.clone()))
            })
            .collect()
    }

    fn draw_sort_bar(&self, frame: &mut Frame, area: Rect) {
        let arrow = if self.ascending { "↑" } else { "↓" };
        let sort_label = format!("[s:{} {}]", self.sort_column.label(), arrow);

        let mut spans = vec![
            Span::styled(
                sort_label,
                Style::default().fg(Color::Yellow),
            ),
        ];

        if !self.selected_indices.is_empty() {
            spans.push(Span::styled(
                format!(" [{}sel]", self.selected_indices.len()),
                Style::default().fg(Color::Rgb(255, 165, 0)),
            ));
        }

        spans.push(Span::styled(
            format!(" {} packages", self.packages.len()),
            Style::default().fg(Color::DarkGray),
        ));

        let line = Line::from(spans);
        frame.render_widget(line, area);
    }

    pub(crate) fn draw_in_area(&mut self, frame: &mut Frame, sort_area: Rect, table_area: Rect) {
        // Draw sort bar
        self.draw_sort_bar(frame, sort_area);

        // Build sort arrow helper
        let sort_arrow = |col: SortColumn| -> &str {
            if self.sort_column == col {
                if self.ascending { " ↑" } else { " ↓" }
            } else {
                ""
            }
        };

        // Build rows
        let mut rows = Vec::new();
        for (display_idx, &pkg_idx) in self.sorted_indices.iter().enumerate() {
            let package = &self.packages[pkg_idx];
            let is_selected = self.selected_indices.contains(&pkg_idx);

            let marker = if is_selected { "*" } else { " " };

            let deps_count = package.depends.len().to_string();
            let size_str = format_size(package.size);
            let date_str = package
                .install_date
                .filter(|&d| d > 0)
                .map(format_timestamp)
                .unwrap_or_else(|| "-".to_string());

            let _ = display_idx; // used for iteration

            rows.push(Row::new(vec![
                Cell::from(format!("{}{}", marker, package.name)),
                Cell::from(size_str),
                Cell::from(date_str),
                Cell::from(deps_count),
            ]));
        }

        // Header with sort arrows
        let header = Row::new(vec![
            Cell::from(Line::from(format!("name{}", sort_arrow(SortColumn::Name)))),
            Cell::from(Line::from(format!("size{}", sort_arrow(SortColumn::Size)))),
            Cell::from(Line::from(format!("installed{}", sort_arrow(SortColumn::InstallDate)))),
            Cell::from(Line::from(format!("deps{}", sort_arrow(SortColumn::DepsCount)))),
        ])
        .style(Style::new().bold().fg(Color::Magenta));

        let widths = [
            Constraint::Fill(1),    // name
            Constraint::Length(12), // size
            Constraint::Length(12), // install date
            Constraint::Length(6),  // deps count
        ];

        let border_color = if self.active {
            self.theme.active
        } else {
            self.theme.inactive
        };

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::bordered().border_style(Style::default().fg(border_color)))
            .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(table, table_area, &mut self.state);
    }
}

impl Component for InstalledTable {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        if !self.active {
            return Ok(None);
        }

        let mut actions = Vec::new();

        if config::key_matches(key_event, &self.keys.cycle_sort) {
            self.sort_column = self.sort_column.next();
            tracing::debug!(column = ?self.sort_column, "cycling sort column");
            let selected_pkg_idx = self.selected_package_index();
            self.resort();
            if let Some(pkg_idx) = selected_pkg_idx {
                if let Some(new_pos) = self.sorted_indices.iter().position(|&i| i == pkg_idx) {
                    self.state.select(Some(new_pos));
                }
            }
        } else if config::key_matches(key_event, &self.keys.toggle_sort_direction) {
            self.ascending = !self.ascending;
            tracing::debug!(ascending = self.ascending, "toggling sort direction");
            let selected_pkg_idx = self.selected_package_index();
            self.resort();
            if let Some(pkg_idx) = selected_pkg_idx {
                if let Some(new_pos) = self.sorted_indices.iter().position(|&i| i == pkg_idx) {
                    self.state.select(Some(new_pos));
                }
            }
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
        } else if config::key_matches(key_event, &self.keys.remove) {
            if let Some(package) = self.get_selected_package() {
                actions.push(Action::RemovePackage {
                    name: package.name.clone(),
                    source: package.source.clone(),
                });
            }
        } else if config::key_matches(key_event, &self.keys.batch_remove) {
            let selected = self.get_selected_packages();
            if !selected.is_empty() {
                tracing::info!(count = selected.len(), "batch remove from installed table");
                actions.push(Action::RemovePackages { packages: selected });
            } else if let Some(package) = self.get_selected_package() {
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
            Event::PackageRemoved(name) => {
                tracing::debug!(name, "removing package from installed table");
                if let Some(idx) = self.packages.iter().position(|p| p.name == *name) {
                    self.packages.remove(idx);
                    // Clean up selection references
                    self.selected_indices.remove(&idx);
                    // Fix shifted indices in selected_indices
                    self.selected_indices = self
                        .selected_indices
                        .iter()
                        .map(|&i| if i > idx { i - 1 } else { i })
                        .collect();
                    self.resort();
                    // Clamp selection
                    if !self.sorted_indices.is_empty() {
                        let current = self.state.selected().unwrap_or(0);
                        if current >= self.sorted_indices.len() {
                            self.state.select(Some(self.sorted_indices.len() - 1));
                        }
                    } else {
                        self.state.select(None);
                    }
                }
            }
            Event::PackagesRemoved(names) => {
                tracing::debug!(count = names.len(), "removing packages from installed table");
                // Remove in reverse order to preserve indices
                let mut indices_to_remove: Vec<usize> = names
                    .iter()
                    .filter_map(|name| self.packages.iter().position(|p| &p.name == name))
                    .collect();
                indices_to_remove.sort_unstable();
                indices_to_remove.reverse();
                for idx in indices_to_remove {
                    self.packages.remove(idx);
                }
                self.selected_indices.clear();
                self.resort();
                if !self.sorted_indices.is_empty() {
                    let current = self.state.selected().unwrap_or(0);
                    if current >= self.sorted_indices.len() {
                        self.state.select(Some(self.sorted_indices.len() - 1));
                    }
                } else {
                    self.state.select(None);
                }
            }
            Event::PackageInstalled(name) => {
                // Package might have been reinstalled; ignore if already present
                if !self.packages.iter().any(|p| p.name == *name) {
                    tracing::debug!(name, "package installed but not in installed table (would need reload)");
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn draw(&mut self, _frame: &mut Frame, _area: &Rect) -> eyre::Result<()> {
        // Rendering is handled by draw_in_area() called from app.rs
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
                tracing::trace!("installed table scroll up");
                self.previous();
                if let Some(package) = self.get_selected_package() {
                    actions.push(Action::SelectPackage(Box::new(package.clone())));
                }
            }
            MouseEventKind::ScrollDown => {
                tracing::trace!("installed table scroll down");
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
