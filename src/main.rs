mod app;
mod tui;
mod theme;
mod pacman;
mod components;

use app::App;
use std::io;

fn main() -> io::Result<()> {
    let mut app = App::new()?;
    app.run()
}
