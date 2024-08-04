mod action;
mod app;
mod components;
mod event;
mod pacman;
mod panic_hook;
mod theme;
mod tui;

use crate::app::App;

use color_eyre::eyre;

fn main() -> eyre::Result<()> {
    panic_hook::init()?;

    let mut app = App::new()?;
    app.run()?;

    Ok(())
}
