use std::ffi::c_void;

use nuklear::Nuklear;

use crate::hooks::Hooks;

pub extern "C" fn hk_poll_event(event: *mut c_void) -> i32 {
    let rc = Hooks::poll_event()
        .original
        .call_poll_event(event as _)
        .expect("Invalid PollEvent function signature");

    Nuklear::handle_menu_show_hide(event);

    if rc != 0 && Nuklear::handle_event(event) && Nuklear::should_draw() {
        Nuklear::capture_input(event);
        return rc;
    }

    rc
}
