use std::panic;

use crate::tui::TUI;

use color_eyre::eyre;

pub(crate) fn init() -> eyre::Result<()> {
    let hook_builder = color_eyre::config::HookBuilder::default();
    let (panic_hook, eyre_hook) = hook_builder.into_hooks();

    let panic_hook = panic_hook.into_panic_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = TUI::exit();
        panic_hook(panic_info);
    }));

    let eyre_hook = eyre_hook.into_eyre_hook();
    eyre::set_hook(Box::new(move |error| {
        let _ = TUI::exit();
        eyre_hook(error)
    }))?;

    Ok(())
}
