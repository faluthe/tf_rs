use std::ffi::c_void;

use log::info;
use nuklear_sys::*;

use crate::hooks::Hooks;

pub extern "C" fn hk_poll_event(event: *mut c_void) -> i32 {
    let event = event as *mut SDL_Event;

    let rc = Hooks::poll_event()
        .original
        .call_poll_event(event as _)
        .expect("Invalid PollEvent function signature");

    info!("In hk_poll_event");

    if rc != 0 && unsafe { nk_sdl_handle_event(event) } != 0 {
        (unsafe { *event }).type_ = 0;

        return rc;
    }

    rc
}
