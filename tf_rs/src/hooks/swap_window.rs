use std::ffi::c_void;

use nuklear::Nuklear;

use crate::{
    config::Config,
    features::{menu, spectator_list},
    helpers,
    hooks::Hooks,
    interfaces::Interfaces,
};

pub extern "C" fn hk_swap_window(window: *mut c_void) -> i32 {
    let nuklear = Nuklear::get_or_init(window);

    // Close the input window opened by the previous frame's swap (or init),
    // so that all poll_event/nk_sdl_handle_event calls this frame are committed.
    nuklear.input_end();

    if Nuklear::should_draw() {
        menu::draw(&nuklear);
    }

    if Interfaces::engine_client().is_in_game() {
        if let Some(localplayer) = helpers::get_localplayer() {
            let config = Config::read();

            spectator_list::draw(&localplayer, &config, &nuklear);
        }
    }

    nuklear.render();

    let rc = Hooks::swap_window()
        .original
        .call_swap_window(window)
        .expect("Invalid SwapWindow function signature");

    // Open the input window for the next frame's poll_event calls.
    nuklear.input_begin();

    rc
}
