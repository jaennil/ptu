use std::io::Stdout;
use std::ops::DerefMut;
use std::{io, ops::Deref};

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{terminal, ExecutableCommand as _},
};

type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

pub struct TUI {
    terminal: Terminal,
    // pub ui: UI,
    raw_mode: bool,
    alternate_screen: bool,
}

impl TUI {
    pub fn new(terminal: Terminal) -> Self {
        // let ui = UI::new();
        Self {
            terminal,
            // ui,
            raw_mode: false,
            alternate_screen: false,
        }
    }

    pub fn enable_raw_mode(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()?;
        self.raw_mode = true;
        Ok(())
    }

    pub fn disable_raw_mode(&mut self) -> io::Result<()> {
        terminal::disable_raw_mode()?;
        self.raw_mode = false;
        Ok(())
    }

    pub fn enter_alternate_screen(&mut self) -> io::Result<()> {
        self.terminal
            .backend_mut()
            .execute(terminal::EnterAlternateScreen)?;
        self.alternate_screen = true;
        Ok(())
    }

    pub fn leave_alternate_screen(&mut self) -> io::Result<()> {
        self.terminal
            .backend_mut()
            .execute(terminal::LeaveAlternateScreen)?;
        self.alternate_screen = false;
        Ok(())
    }

    pub fn with_alternate_screen(mut self) -> io::Result<Self> {
        self.enter_alternate_screen()?;
        Ok(self)
    }

    pub fn with_raw_mode(mut self) -> io::Result<Self> {
        self.enable_raw_mode()?;
        Ok(self)
    }

    pub fn init(&mut self) -> io::Result<()> {
        self.enable_raw_mode()?;
        self.enter_alternate_screen()
    }

    pub fn restore(&mut self) -> io::Result<()> {
        if self.raw_mode {
            self.disable_raw_mode()?;
        }

        if self.alternate_screen {
            self.leave_alternate_screen()?;
        }

        Ok(())
    }

    pub fn suspend<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(),
    {
        self.restore()?;
        f();
        // TODO: enable only previously enabled features
        self.init()?;
        self.terminal.clear()
    }

    // pub fn draw(&mut self) -> io::Result<ratatui::CompletedFrame> {
    //     self.terminal.draw(|frame| self.ui.render(frame))
    // }
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
