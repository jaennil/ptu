use color_eyre::eyre;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Text,
    widgets::{Block, Cell, Row, Table},
    Frame,
};
use textwrap::wrap;

use crate::{
    components::Component, event::Event, layout::{LABEL_WIDTH, LEFT_PANEL_PERCENT},
    pacman::{format_size, Package},
    theme::Theme,
};

#[derive(Default)]
pub(crate) struct PackageInfo {
    package: Package,
    theme: Theme,
}

fn create_row<'a>(label: &'a str, value: &'a str, width: usize) -> Row<'a> {
    let wrapped: Vec<String> = wrap(value, width).iter().map(|s| s.to_string()).collect();
    let height = wrapped.len().max(1) as u16;
    let wrapped_text = wrapped.join("\n");
    Row::new(vec![
        Cell::new(Text::raw(label)),
        Cell::new(Text::raw(wrapped_text)),
    ])
    .height(height)
}

impl Component for PackageInfo {
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

        let mut rows = vec![
            create_row("description", &self.package.description, value_width),
            create_row("version", &self.package.version, value_width),
        ];
        if self.package.size > 0 {
            rows.push(create_row("size", &size_str, value_width));
        }
        if !self.package.licenses.is_empty() {
            rows.push(create_row("licenses", &licenses_str, value_width));
        }
        if !self.package.depends.is_empty() {
            rows.push(create_row("depends", &depends_str, value_width));
        }
        rows.extend([
            create_row("arch", &self.package.arch, value_width),
            create_row("url", &self.package.url, value_width),
            create_row("packager", &self.package.packager, value_width),
            create_row("base", &self.package.base, value_width),
            create_row("filename", &self.package.filename, value_width),
            create_row("md5sum", &self.package.md5sum, value_width),
            create_row("sha256sum", &self.package.sha256sum, value_width),
        ]);
        let widths = [Constraint::Length(LABEL_WIDTH), Constraint::Percentage(100)];
        let table = Table::new(rows, widths)
            .block(Block::bordered().border_style(Style::default().fg(self.theme.active)));
        frame.render_widget(table, area);
        Ok(())
    }

    fn update(&mut self, event: &Event) -> eyre::Result<()> {
        if let Event::PackageSelected(package) = event {
            self.package = (**package).clone();
        }

        Ok(())
    }
}
