use std::ffi::c_void;

use log::info;

use crate::hooks::Hooks;

pub extern "C" fn hk_swap_window(window: *mut c_void) -> i32 {
    let rc = Hooks::swap_window()
        .original
        .call_swap_window(window)
        .expect("Invalid SwapWindow function signature");

    info!("In hk_swap_window");

    rc
}
