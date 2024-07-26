use std::panic;

use crate::tui::TUI;

pub fn init() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let mut tui = TUI::new().unwrap();
        let _ = tui.restore();
        original_hook(panic_info);
    }));
}
