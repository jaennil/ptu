use std::io::Stdout;
use std::ops::DerefMut;
use std::{io, ops::Deref};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::crossterm::{terminal, ExecutableCommand as _};

type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

pub struct TUI {
    terminal: Terminal,
}

impl TUI {
    pub fn new() -> io::Result<Self> {
        let writer = io::stdout();
        let backend = CrosstermBackend::new(writer);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    pub fn init(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()?;
        self.terminal.backend_mut().execute(EnterAlternateScreen)?;
        Ok(())
    }

    pub fn restore(&mut self) -> io::Result<()> {
        if terminal::is_raw_mode_enabled()? {
            terminal::disable_raw_mode()?;
            self.terminal.backend_mut().execute(LeaveAlternateScreen)?;
        }

        Ok(())
    }

    pub fn suspend<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(),
    {
        self.restore()?;
        f();
        self.init()?;
        self.terminal.clear()
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
