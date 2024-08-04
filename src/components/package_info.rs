use color_eyre::eyre;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Row, Table},
    Frame,
};

use crate::{components::Component, event::Event, pacman::Package, theme::Theme};

#[derive(Default)]
pub(crate) struct PackageInfo {
    package: Package,
    theme: Theme,
}

impl Component for PackageInfo {
    fn draw(&mut self, frame: &mut Frame, area: &Rect) -> eyre::Result<()> {
        let area = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(*area)[1];
        let rows = [
            Row::new(vec!["description", &self.package.description]),
            Row::new(vec!["version", &self.package.version]),
            Row::new(vec!["filename", &self.package.filename]),
            Row::new(vec!["base", &self.package.base]),
            Row::new(vec!["url", &self.package.url]),
            Row::new(vec!["packager", &self.package.packager]),
            Row::new(vec!["md5sum", &self.package.md5sum]),
            Row::new(vec!["sha256sum", &self.package.sha256sum]),
            Row::new(vec!["arch", &self.package.arch]),
        ];
        let widths = [Constraint::Length(15), Constraint::Percentage(100)];
        let table = Table::new(rows, widths)
            .block(Block::bordered().border_style(Style::default().fg(self.theme.active)));
        frame.render_widget(table, area);
        Ok(())
    }

    fn update(&mut self, event: &Event) -> eyre::Result<()> {
        match event {
            Event::PackageSelected(package) => {
                self.package = package.clone();
            }
            _ => {}
        }

        Ok(())
    }
}
