mod action;
mod app;
mod components;
mod errors;
mod pacman;
mod theme;
mod tui;

use app::App;
use std::io;

// TODO: mb try tui-term
// TODO: mb try tui-input
fn main() -> io::Result<()> {
    errors::init();

    let mut app = App::new()?;
    app.run()
}
