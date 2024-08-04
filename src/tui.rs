use std::{
    io,
    ops::{Deref, DerefMut},
};

use color_eyre::eyre;
use ratatui::crossterm::{self, terminal};

type Terminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>;

pub(crate) struct TUI {
    terminal: Terminal,
}

impl TUI {
    pub(crate) fn new() -> eyre::Result<Self> {
        let writer = io::stdout();
        let backend = ratatui::backend::CrosstermBackend::new(writer);
        let terminal = ratatui::Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub(crate) fn enter() -> eyre::Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(io::stdout(), terminal::EnterAlternateScreen)?;
        Ok(())
    }

    pub(crate) fn exit() -> eyre::Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(io::stdout(), terminal::LeaveAlternateScreen)?;
        Ok(())
    }

    pub(crate) fn suspend<F>(&mut self, f: F) -> eyre::Result<()>
    where
        F: FnOnce() -> eyre::Result<()>,
    {
        Self::exit()?;
        f()?;
        Self::enter()?;
        self.clear()?;
        Ok(())
    }
}

impl Deref for TUI {
    type Target = Terminal;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for TUI {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}
