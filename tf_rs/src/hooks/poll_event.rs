use std::ffi::c_void;

use log::info;

use crate::hooks::Hooks;

pub extern "C" fn hk_poll_event(event: *mut c_void) -> i32 {
    let rc = Hooks::poll_event()
        .original
        .call_poll_event(event)
        .expect("Invalid PollEvent function signature");

    info!("In hk_poll_event");

    rc
}
