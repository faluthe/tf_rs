use std::ffi::{CString, c_void};

use log::info;
use nuklear::{Nuklear, Rect, flags::PanelFlags, flags::TextAlignment};

use crate::hooks::Hooks;

pub extern "C" fn hk_swap_window(window: *mut c_void) -> i32 {
    info!("hk_swap_window called with window: {:?}", window);

    let nuklear = Nuklear::begin(
        "TF_RS",
        PanelFlags::BORDER | PanelFlags::MOVABLE | PanelFlags::TITLE,
        Rect {
            x: 200.0,
            y: 200.0,
            w: 500.0,
            h: 600.0,
        },
        window,
    );

    nuklear
        .row_dynamic(40.0, 1)
        .label(
            CString::new("Welcome to TF_RS!").unwrap(),
            TextAlignment::LEFT,
        )
        .render();

    nuklear.input_begin();
    let rc = Hooks::swap_window()
        .original
        .call_swap_window(window)
        .expect("Invalid SwapWindow function signature");
    nuklear.input_end();

    rc
}
