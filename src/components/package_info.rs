use color_eyre::eyre;
use ratatui::{
    crossterm::event::{KeyEvent, MouseEvent, MouseEventKind},
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::{
    action::Action,
    components::Component,
    config::{self, PackageInfoKeys},
    event::Event,
    layout::{LABEL_WIDTH, LEFT_PANEL_PERCENT},
    pacman::{format_size, format_timestamp, Package},
    theme::Theme,
};

const SECTION_EXPAND_THRESHOLD: usize = 5;

enum InfoRow {
    Scalar { label: String, value: String },
    Section { label: String, items: Vec<String>, expanded: bool },
}

pub(crate) struct PackageInfo {
    package: Package,
    theme: Theme,
    active: bool,
    rows: Vec<InfoRow>,
    cursor: usize,
    scroll_offset: usize,
    visible_height: usize,
    keys: PackageInfoKeys,
}

impl PackageInfo {
    pub(crate) fn new(keys: PackageInfoKeys) -> Self {
        Self {
            package: Package::default(),
            theme: Theme::default(),
            active: false,
            rows: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            visible_height: 0,
            keys,
        }
    }

    fn build_rows(package: &Package) -> Vec<InfoRow> {
        let mut rows = Vec::new();

        // Scalar rows
        rows.push(InfoRow::Scalar {
            label: "description".to_string(),
            value: package.description.clone(),
        });
        rows.push(InfoRow::Scalar {
            label: "version".to_string(),
            value: package.version.clone(),
        });
        if let Some(ref update_ver) = package.update_version {
            rows.push(InfoRow::Scalar {
                label: "update".to_string(),
                value: update_ver.clone(),
            });
        }
        if package.size > 0 {
            rows.push(InfoRow::Scalar {
                label: "size".to_string(),
                value: format_size(package.size),
            });
        }
        if package.build_date.is_some() {
            rows.push(InfoRow::Scalar {
                label: "build date".to_string(),
                value: package.build_date.map(format_timestamp).unwrap_or_default(),
            });
        }
        if package.install_date.is_some() {
            rows.push(InfoRow::Scalar {
                label: "install date".to_string(),
                value: package.install_date.map(format_timestamp).unwrap_or_default(),
            });
        }
        // AUR-specific fields
        if let Some(votes) = package.votes {
            rows.push(InfoRow::Scalar {
                label: "votes".to_string(),
                value: votes.to_string(),
            });
        }
        if let Some(popularity) = package.popularity {
            rows.push(InfoRow::Scalar {
                label: "popularity".to_string(),
                value: format!("{:.2}", popularity),
            });
        }
        if let Some(out_of_date) = package.out_of_date {
            rows.push(InfoRow::Scalar {
                label: "out of date".to_string(),
                value: format_timestamp(out_of_date),
            });
        }
        if let Some(submitted) = package.first_submitted {
            rows.push(InfoRow::Scalar {
                label: "submitted".to_string(),
                value: format_timestamp(submitted),
            });
        }
        if let Some(modified) = package.last_modified {
            rows.push(InfoRow::Scalar {
                label: "updated".to_string(),
                value: format_timestamp(modified),
            });
        }
        if package.arch != "-" && !package.arch.is_empty() {
            rows.push(InfoRow::Scalar {
                label: "arch".to_string(),
                value: package.arch.clone(),
            });
        }
        if !package.url.is_empty() {
            rows.push(InfoRow::Scalar {
                label: "url".to_string(),
                value: package.url.clone(),
            });
        }
        rows.push(InfoRow::Scalar {
            label: "packager".to_string(),
            value: package.packager.clone(),
        });
        if package.base != "-" && !package.base.is_empty() {
            rows.push(InfoRow::Scalar {
                label: "base".to_string(),
                value: package.base.clone(),
            });
        }
        if package.filename != "-" && !package.filename.is_empty() {
            rows.push(InfoRow::Scalar {
                label: "filename".to_string(),
                value: package.filename.clone(),
            });
        }
        if package.md5sum != "-" && !package.md5sum.is_empty() {
            rows.push(InfoRow::Scalar {
                label: "md5sum".to_string(),
                value: package.md5sum.clone(),
            });
        }
        if package.sha256sum != "-" && !package.sha256sum.is_empty() {
            rows.push(InfoRow::Scalar {
                label: "sha256sum".to_string(),
                value: package.sha256sum.clone(),
            });
        }

        // Section rows
        let sections: Vec<(&str, &Vec<String>, bool)> = vec![
            ("licenses", &package.licenses, false),
            ("depends", &package.depends, false),
            ("optdepends", &package.optdepends, false),
            ("required_by", &package.required_by, false),
            ("groups", &package.groups, false),
            ("provides", &package.provides, false),
            ("conflicts", &package.conflicts, false),
            ("matched files", &package.matched_files, false),
            ("files", &package.files, true),
        ];

        for (label, items, is_files) in sections {
            if !items.is_empty() {
                let expanded = if is_files {
                    false // files always collapsed
                } else {
                    items.len() <= SECTION_EXPAND_THRESHOLD
                };
                rows.push(InfoRow::Section {
                    label: label.to_string(),
                    items: items.clone(),
                    expanded,
                });
            }
        }

        rows
    }

    fn cursor_down(&mut self) {
        if !self.rows.is_empty() && self.cursor < self.rows.len() - 1 {
            self.cursor += 1;
            tracing::trace!(cursor = self.cursor, "info cursor down");
            self.ensure_cursor_visible();
        }
    }

    fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            tracing::trace!(cursor = self.cursor, "info cursor up");
            self.ensure_cursor_visible();
        }
    }

    fn toggle_section(&mut self) {
        if let Some(InfoRow::Section { expanded, label, items, .. }) = self.rows.get_mut(self.cursor) {
            *expanded = !*expanded;
            tracing::debug!(label, expanded = *expanded, count = items.len(), "toggled section");
            self.ensure_cursor_visible();
        }
    }

    /// Calculate the visual line offset where a given row index starts
    fn visual_line_for_row(&self, row_idx: usize) -> usize {
        let mut line = 0;
        for (i, row) in self.rows.iter().enumerate() {
            if i == row_idx {
                return line;
            }
            line += self.row_visual_height(row);
        }
        line
    }

    fn row_visual_height(&self, row: &InfoRow) -> usize {
        match row {
            InfoRow::Scalar { .. } => 1,
            InfoRow::Section { items, expanded, .. } => {
                if *expanded {
                    1 + items.len()
                } else {
                    1
                }
            }
        }
    }

    fn ensure_cursor_visible(&mut self) {
        if self.visible_height == 0 {
            return;
        }

        let cursor_start = self.visual_line_for_row(self.cursor);
        let cursor_end = cursor_start + self.row_visual_height(&self.rows[self.cursor]);

        // If cursor is above viewport, scroll up
        if cursor_start < self.scroll_offset {
            self.scroll_offset = cursor_start;
        }
        // If cursor bottom is below viewport, scroll down
        if cursor_end > self.scroll_offset + self.visible_height {
            self.scroll_offset = cursor_end.saturating_sub(self.visible_height);
        }
    }
}

impl Component for PackageInfo {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        if !self.active {
            return Ok(None);
        }

        if config::key_matches(key_event, &self.keys.scroll_down) {
            self.cursor_down();
        } else if config::key_matches(key_event, &self.keys.scroll_up) {
            self.cursor_up();
        } else if config::key_matches(key_event, &self.keys.page_down) {
            for _ in 0..10 {
                self.cursor_down();
            }
        } else if config::key_matches(key_event, &self.keys.page_up) {
            for _ in 0..10 {
                self.cursor_up();
            }
        } else if config::key_matches(key_event, &self.keys.toggle_section) {
            self.toggle_section();
        } else if config::key_matches(key_event, &self.keys.open_url) {
            if !self.package.url.is_empty() {
                tracing::info!(url = %self.package.url, "opening package URL in browser");
                return Ok(Some(vec![Action::OpenUrl(self.package.url.clone())]));
            } else {
                tracing::debug!("no URL available for this package");
            }
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: &Rect) -> eyre::Result<()> {
        let area = Layout::horizontal([
            Constraint::Percentage(LEFT_PANEL_PERCENT),
            Constraint::Percentage(100 - LEFT_PANEL_PERCENT),
        ])
        .split(*area)[1];

        let border_color = if self.active {
            self.theme.active
        } else {
            self.theme.inactive
        };

        let block = Block::bordered().border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        self.visible_height = inner.height as usize;

        let label_width = LABEL_WIDTH as usize;
        let highlight_style = if self.active {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        // Build visual lines
        let mut lines: Vec<Line> = Vec::new();
        let mut row_to_visual: Vec<usize> = Vec::new(); // maps row index -> starting visual line

        for (row_idx, row) in self.rows.iter().enumerate() {
            row_to_visual.push(lines.len());
            match row {
                InfoRow::Scalar { label, value } => {
                    let style = if row_idx == self.cursor { highlight_style } else { Style::default() };
                    let padded_label = format!("{:<width$}", label, width = label_width);
                    lines.push(Line::from(vec![
                        Span::styled(padded_label, style),
                        Span::styled(value.as_str(), style),
                    ]));
                }
                InfoRow::Section { label, items, expanded } => {
                    let style = if row_idx == self.cursor { highlight_style } else { Style::default() };
                    let arrow = if *expanded { "\u{25be}" } else { "\u{25b8}" };
                    let padded_label = format!("{:<width$}", label, width = label_width);
                    lines.push(Line::from(vec![
                        Span::styled(padded_label, style),
                        Span::styled(format!("({}) {}", items.len(), arrow), style),
                    ]));
                    if *expanded {
                        let indent = " ".repeat(label_width);
                        for item in items {
                            lines.push(Line::from(vec![
                                Span::raw(indent.clone()),
                                Span::raw(item.as_str()),
                            ]));
                        }
                    }
                }
            }
        }

        let total_lines = lines.len();

        // Apply scroll offset - skip lines before viewport
        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.scroll_offset)
            .take(self.visible_height)
            .collect();

        let paragraph = Paragraph::new(visible_lines);
        frame.render_widget(paragraph, inner);

        // Render scrollbar if content overflows
        if total_lines > self.visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("\u{2191}"))
                .end_symbol(Some("\u{2193}"));
            let mut scrollbar_state = ScrollbarState::new(total_lines)
                .position(self.scroll_offset);
            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin { vertical: 1, horizontal: 0 }),
                &mut scrollbar_state,
            );
        }

        Ok(())
    }

    fn update(&mut self, event: &Event) -> eyre::Result<()> {
        if let Event::PackageSelected(package) = event {
            self.package = (**package).clone();
            self.rows = Self::build_rows(&self.package);
            self.cursor = 0;
            self.scroll_offset = 0;
            tracing::debug!(
                name = self.package.name,
                row_count = self.rows.len(),
                "package info rows built"
            );
        }

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

        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                tracing::trace!("package info scroll up");
                self.cursor_up();
            }
            MouseEventKind::ScrollDown => {
                tracing::trace!("package info scroll down");
                self.cursor_down();
            }
            _ => {}
        }

        Ok(None)
    }
}
