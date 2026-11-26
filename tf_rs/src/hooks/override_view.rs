use std::ffi::c_void;

use crate::{config::Config, helpers, hooks::Hooks, interfaces::Interfaces, types::ViewSetup};

pub extern "C" fn hk_override_view(this: *mut c_void, setup: *mut ViewSetup) {
    Hooks::override_view()
        .call_override_view(this, setup)
        .expect("Invalid OverrideView function signature");

    if !Interfaces::engine_client().is_in_game() {
        return;
    }

    let Some(localplayer) = helpers::get_localplayer() else {
        return;
    };

    if Config::read().no_zoom {
        unsafe {
            (*setup).fov = localplayer.fov() as f32;
        }
    }
    if let Some(setup) = unsafe { setup.as_ref() } {
        helpers::view::record(setup);
    }
}
