use color_eyre::eyre;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind},
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Text,
    widgets::{Block, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState},
    Frame,
};
use textwrap::wrap;

use crate::{
    action::Action,
    components::Component,
    event::Event,
    layout::{LABEL_WIDTH, LEFT_PANEL_PERCENT},
    pacman::{format_size, format_timestamp, Package},
    theme::Theme,
};

#[derive(Default)]
pub(crate) struct PackageInfo {
    package: Package,
    theme: Theme,
    active: bool,
    scroll_state: TableState,
    total_rows: usize,
    total_visual_height: u16,
}

fn create_row<'a>(label: &'a str, value: &'a str, width: usize) -> (Row<'a>, u16) {
    let wrapped: Vec<String> = wrap(value, width).iter().map(|s| s.to_string()).collect();
    let height = wrapped.len().max(1) as u16;
    let wrapped_text = wrapped.join("\n");
    let row = Row::new(vec![
        Cell::new(Text::raw(label)),
        Cell::new(Text::raw(wrapped_text)),
    ])
    .height(height);
    (row, height)
}

impl PackageInfo {
    fn scroll_down(&mut self) {
        let max_offset = self.total_rows.saturating_sub(1);
        let current = self.scroll_state.offset();
        if current < max_offset {
            *self.scroll_state.offset_mut() = current + 1;
        }
    }

    fn scroll_up(&mut self) {
        let current = self.scroll_state.offset();
        *self.scroll_state.offset_mut() = current.saturating_sub(1);
    }
}

impl Component for PackageInfo {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        if !self.active {
            return Ok(None);
        }

        match key_event {
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Char('j'),
                ..
            } => self.scroll_down(),
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Char('k'),
                ..
            } => self.scroll_up(),
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('d'),
                ..
            } => {
                for _ in 0..10 {
                    self.scroll_down();
                }
            }
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('u'),
                ..
            } => {
                for _ in 0..10 {
                    self.scroll_up();
                }
            }
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Char('o'),
                ..
            } => {
                if !self.package.url.is_empty() {
                    tracing::info!(url = %self.package.url, "opening package URL in browser");
                    return Ok(Some(vec![Action::OpenUrl(self.package.url.clone())]));
                } else {
                    tracing::debug!("no URL available for this package");
                }
            }
            _ => {}
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: &Rect) -> eyre::Result<()> {
        let area = Layout::horizontal([
            Constraint::Percentage(LEFT_PANEL_PERCENT),
            Constraint::Percentage(100 - LEFT_PANEL_PERCENT),
        ])
        .split(*area)[1];

        let border_padding = 3u16;
        let value_width = area.width.saturating_sub(LABEL_WIDTH + border_padding) as usize;

        let size_str = format_size(self.package.size);
        let licenses_str = self.package.licenses.join(", ");
        let depends_str = self.package.depends.join(", ");
        let optdepends_str = self.package.optdepends.join(", ");
        let groups_str = self.package.groups.join(", ");
        let provides_str = self.package.provides.join(", ");
        let conflicts_str = self.package.conflicts.join(", ");
        let build_date_str = self.package.build_date.map(format_timestamp).unwrap_or_default();
        let votes_str = self.package.votes.map(|v| v.to_string()).unwrap_or_default();
        let popularity_str = self.package.popularity.map(|p| format!("{:.2}", p)).unwrap_or_default();
        let out_of_date_str = self.package.out_of_date.map(format_timestamp).unwrap_or_default();
        let submitted_str = self.package.first_submitted.map(format_timestamp).unwrap_or_default();
        let updated_str = self.package.last_modified.map(format_timestamp).unwrap_or_default();

        let mut rows_with_heights: Vec<(Row, u16)> = Vec::new();

        rows_with_heights.push(create_row("description", &self.package.description, value_width));
        rows_with_heights.push(create_row("version", &self.package.version, value_width));
        if self.package.size > 0 {
            rows_with_heights.push(create_row("size", &size_str, value_width));
        }
        if !self.package.licenses.is_empty() {
            rows_with_heights.push(create_row("licenses", &licenses_str, value_width));
        }
        if !self.package.depends.is_empty() {
            rows_with_heights.push(create_row("depends", &depends_str, value_width));
        }
        if !self.package.optdepends.is_empty() {
            rows_with_heights.push(create_row("optdepends", &optdepends_str, value_width));
        }
        if !self.package.groups.is_empty() {
            rows_with_heights.push(create_row("groups", &groups_str, value_width));
        }
        if !self.package.provides.is_empty() {
            rows_with_heights.push(create_row("provides", &provides_str, value_width));
        }
        if !self.package.conflicts.is_empty() {
            rows_with_heights.push(create_row("conflicts", &conflicts_str, value_width));
        }
        if self.package.build_date.is_some() {
            rows_with_heights.push(create_row("build date", &build_date_str, value_width));
        }
        // AUR-specific fields
        if self.package.votes.is_some() {
            rows_with_heights.push(create_row("votes", &votes_str, value_width));
        }
        if self.package.popularity.is_some() {
            rows_with_heights.push(create_row("popularity", &popularity_str, value_width));
        }
        if self.package.out_of_date.is_some() {
            rows_with_heights.push(create_row("out of date", &out_of_date_str, value_width));
        }
        if self.package.first_submitted.is_some() {
            rows_with_heights.push(create_row("submitted", &submitted_str, value_width));
        }
        if self.package.last_modified.is_some() {
            rows_with_heights.push(create_row("updated", &updated_str, value_width));
        }
        if self.package.arch != "-" && !self.package.arch.is_empty() {
            rows_with_heights.push(create_row("arch", &self.package.arch, value_width));
        }
        if !self.package.url.is_empty() {
            rows_with_heights.push(create_row("url", &self.package.url, value_width));
        }
        rows_with_heights.push(create_row("packager", &self.package.packager, value_width));
        if self.package.base != "-" && !self.package.base.is_empty() {
            rows_with_heights.push(create_row("base", &self.package.base, value_width));
        }
        if self.package.filename != "-" && !self.package.filename.is_empty() {
            rows_with_heights.push(create_row("filename", &self.package.filename, value_width));
        }
        if self.package.md5sum != "-" && !self.package.md5sum.is_empty() {
            rows_with_heights.push(create_row("md5sum", &self.package.md5sum, value_width));
        }
        if self.package.sha256sum != "-" && !self.package.sha256sum.is_empty() {
            rows_with_heights.push(create_row("sha256sum", &self.package.sha256sum, value_width));
        }

        self.total_rows = rows_with_heights.len();
        self.total_visual_height = rows_with_heights.iter().map(|(_, h)| h).sum();
        let rows: Vec<Row> = rows_with_heights.into_iter().map(|(r, _)| r).collect();

        let border_color = if self.active {
            self.theme.active
        } else {
            self.theme.inactive
        };

        let widths = [Constraint::Length(LABEL_WIDTH), Constraint::Percentage(100)];
        let table = Table::new(rows, widths)
            .block(Block::bordered().border_style(Style::default().fg(border_color)))
            .row_highlight_style(Style::default());

        frame.render_stateful_widget(table, area, &mut self.scroll_state);

        // Render scrollbar only if content exceeds visible area
        let visible_height = area.height.saturating_sub(2); // subtract borders
        if self.total_visual_height > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state = ScrollbarState::new(self.total_rows)
                .position(self.scroll_state.offset());
            frame.render_stateful_widget(
                scrollbar,
                area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
                &mut scrollbar_state,
            );
        }

        Ok(())
    }

    fn update(&mut self, event: &Event) -> eyre::Result<()> {
        if let Event::PackageSelected(package) = event {
            self.package = (**package).clone();
            *self.scroll_state.offset_mut() = 0; // Reset scroll on new package
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
                self.scroll_up();
            }
            MouseEventKind::ScrollDown => {
                tracing::trace!("package info scroll down");
                self.scroll_down();
            }
            _ => {}
        }

        Ok(None)
    }
}
