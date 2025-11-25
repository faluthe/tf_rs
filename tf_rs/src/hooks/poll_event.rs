use std::ffi::c_void;

use nuklear::{Input, Nuklear, SDL_Scancode};

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
            match Nuklear::get_input_pressed() {
                Input::Key(code) => {
                    globals.aimbot_key_editing = false;
                    if code != SDL_Scancode::SDL_SCANCODE_ESCAPE {
                        config.aimbot.key.is_mouse_button = false;
                        config.aimbot.key.code = code as u32;
                    }
                }
                Input::MouseButton(btn) => {
                    globals.aimbot_key_editing = false;
                    config.aimbot.key.is_mouse_button = true;
                    config.aimbot.key.code = btn;
                }
                Input::None => {}
            }
        }

        Nuklear::capture_input(event);
        return rc;
    }

    if rc != 0 {
        globals.aimbot_key_down =
            Nuklear::is_input_pressed(config.aimbot.key.code, config.aimbot.key.is_mouse_button);
    }

    rc
}
