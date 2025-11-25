use std::ffi::c_void;

use nuklear::{NkKey, Nuklear};

use crate::{features::menu, hooks::Hooks, interfaces::Interfaces};

pub extern "C" fn hk_swap_window(window: *mut c_void) -> i32 {
    let nuklear = Nuklear::get_or_init(window);

    if nuklear.is_draw_key_released(NkKey::Delete) {
        Interfaces::surface().set_cursor_visible(Nuklear::should_draw());
    }

    if Nuklear::should_draw() {
        menu::draw(&nuklear);
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
