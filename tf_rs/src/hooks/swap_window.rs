use std::ffi::c_void;

use nuklear::{
    Key, Nuklear, Rect,
    flags::{PanelFlags, TextAlignment},
};

use crate::{cfg_enabled, config::CONFIG, hooks::Hooks, interfaces::Interfaces};

pub extern "C" fn hk_swap_window(window: *mut c_void) -> i32 {
    let nuklear = Nuklear::get_or_init(window);

    if nuklear.is_draw_key_released(Key::Delete) {
        Interfaces::surface().set_cursor_visible(Nuklear::should_draw());
    }

    if Nuklear::should_draw() {
        draw_menu(&nuklear);
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

fn draw_menu(nk: &Nuklear) {
    if nk.begin(
        "TF_RS",
        PanelFlags::BORDER | PanelFlags::MOVABLE | PanelFlags::TITLE,
        Rect {
            x: 200.0,
            y: 200.0,
            w: 500.0,
            h: 600.0,
        },
    ) {
        nk.row_dynamic(30.0, 1)
            .label("TF_RS Menu", TextAlignment::LEFT)
            .row_dynamic(30.0, 1)
            .checkbox("Bunnyhop", CONFIG.bunnyhop.as_ptr())
            .row_dynamic(30.0, 1)
            .checkbox("ESP", CONFIG.esp.as_ptr())
            .row_dynamic(30.0, 1)
            .checkbox("Aimbot", CONFIG.aimbot.as_ptr());

        if cfg_enabled!(aimbot) {
            nk.row_dynamic(30.0, 1)
                .checkbox("Silent Aim", CONFIG.silent_aim.as_ptr());
        }
    }
    nk.end();
}
