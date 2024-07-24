mod app;
mod tui;
mod theme;
mod pacman;
mod components;
mod action;

use app::App;
use std::io;

// TODO: mb try tui-term
// TODO: mb try tui-input
fn main() -> io::Result<()> {
    let mut app = App::new()?;
    app.run()
}
