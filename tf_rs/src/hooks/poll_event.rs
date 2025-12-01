use std::ffi::c_void;

use nuklear::{Input, Nuklear, SDL_Scancode};

use crate::{
    config::{Config, KeyConfig},
    globals::Globals,
    hooks::Hooks,
};

pub extern "C" fn hk_poll_event(event: *mut c_void) -> i32 {
    let rc = Hooks::poll_event()
        .original
        .call_poll_event(event as _)
        .expect("Invalid PollEvent function signature");

    let mut globals = Globals::write();
    let mut config = Config::write();

    if rc != 0 && Nuklear::handle_event(event) && Nuklear::should_draw() {
        handle_key_edit(&mut globals.aimbot_key_editing, &mut config.aimbot.key);
        handle_key_edit(
            &mut globals.thirdperson_key_editing,
            &mut config.thirdperson,
        );

        Nuklear::capture_input(event);
        return rc;
    }

    if rc != 0 {
        globals.aimbot_key_down =
            Nuklear::is_input_pressed(config.aimbot.key.code, config.aimbot.key.is_mouse_button);
        globals.thirdperson_pressed =
            Nuklear::is_input_pressed(config.thirdperson.code, config.thirdperson.is_mouse_button);
    }

    rc
}

fn handle_key_edit(editing_flag: &mut bool, key_config: &mut KeyConfig) {
    if !*editing_flag {
        return;
    }

    match Nuklear::get_input_pressed() {
        Input::Key(code) => {
            *editing_flag = false;
            if code != SDL_Scancode::SDL_SCANCODE_ESCAPE {
                key_config.is_mouse_button = false;
                key_config.code = code as u32;
            }
        }
        Input::MouseButton(btn) => {
            *editing_flag = false;
            key_config.is_mouse_button = true;
            key_config.code = btn;
        }
        Input::None => {}
    }
}
