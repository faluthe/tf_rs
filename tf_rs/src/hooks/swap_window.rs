use std::ffi::{CString, c_void};

use nuklear::{Nuklear, Rect, flags::PanelFlags, flags::TextAlignment};

use crate::hooks::Hooks;

pub extern "C" fn hk_swap_window(window: *mut c_void) -> i32 {
    let nuklear = Nuklear::get_or_init(window);
    nuklear.make_current();

    if Nuklear::should_draw() {
        if nuklear.begin(
            "TF_RS",
            PanelFlags::BORDER | PanelFlags::MOVABLE | PanelFlags::TITLE,
            Rect {
                x: 200.0,
                y: 200.0,
                w: 500.0,
                h: 600.0,
            },
        ) {
            nuklear
                .row_dynamic(30.0, 2)
                .label(CString::new("TF_RS Menu").unwrap(), TextAlignment::LEFT);
        }
        nuklear.end();
    }

    nuklear.render();

    nuklear.input_begin();
    let rc = Hooks::swap_window()
        .original
        .call_swap_window(window)
        .expect("Invalid SwapWindow function signature");
    nuklear.input_end();

    rc
}
