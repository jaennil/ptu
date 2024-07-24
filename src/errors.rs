use std::panic::{set_hook, take_hook};

use crate::tui::TUI;

pub fn init() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let mut tui = TUI::new().unwrap();
        let _ = tui.restore();
        original_hook(panic_info);
    }));
}
