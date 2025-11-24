use std::ffi::c_void;

use nuklear::{Nuklear, SDL_Scancode};

use crate::{config::Config, globals::Globals, hooks::Hooks};

pub extern "C" fn hk_poll_event(event: *mut c_void) -> i32 {
    let rc = Hooks::poll_event()
        .original
        .call_poll_event(event as _)
        .expect("Invalid PollEvent function signature");

    let mut globals = Globals::write();
    let mut config = Config::write();

    if rc != 0 && Nuklear::handle_event(event) && Nuklear::should_draw() {
        if globals.aimbot_key_editing {
            // TODO: Handle mouse buttons
            if let Some(key) = Nuklear::get_key_pressed() {
                globals.aimbot_key_editing = false;
                if key != SDL_Scancode::SDL_SCANCODE_ESCAPE {
                    config.aimbot.key = key;
                }
            }
        }

        Nuklear::capture_input(event);
        return rc;
    }

    if rc != 0 {
        globals.aimbot_key_down = Nuklear::is_key_pressed(config.aimbot.key);
    }

    rc
}
