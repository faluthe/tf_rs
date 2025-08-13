use std::{ffi::c_void, mem};

use log::info;

use crate::hooks::Hooks;

pub extern "C" fn hk_create_move(this: *mut c_void, sample_time: f32, cmd: *mut c_void) -> i64 {
    let rc = unsafe {
        let og = mem::transmute::<*const c_void, extern "C" fn(*mut c_void, f32, *mut c_void) -> i64>(
            Hooks::create_move().original,
        );
        og(this, sample_time, cmd)
    };

    info!("hk_create_move this={this:?} sample_time={sample_time} cmd={cmd:?} rc={rc}");

    rc
}
