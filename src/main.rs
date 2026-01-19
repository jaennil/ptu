mod action;
mod app;
mod components;
mod event;
mod focus;
mod layout;
mod logging;
mod pacman;
mod panic_hook;
mod theme;
mod tui;

use crate::app::App;

use color_eyre::eyre;

fn main() -> eyre::Result<()> {
    logging::init()?;
    panic_hook::init()?;

    tracing::info!("starting ptu");

    let mut app = App::new()?;
    app.run()?;

    tracing::info!("ptu exited successfully");

    Ok(())
}
