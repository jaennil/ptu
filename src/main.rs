mod app;
mod tui;
mod ui;
mod theme;
mod component;
mod search;
mod home;
mod pacman;
mod table;

use app::App;
use std::io;

fn main() -> io::Result<()> {
    let mut app = App::new()?;
    app.run()
}
