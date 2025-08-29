use std::ffi::c_void;

use log::info;

use crate::hooks::{Hooks, fn_sig::FnSig};

pub extern "C" fn hk_create_move(this: *mut c_void, sample_time: f32, cmd: *mut c_void) -> i64 {
    let rc = {
        let og = match Hooks::create_move().original {
            FnSig::CreateMove(f) => f,
            FnSig::None => {
                info!("Original CreateMove is None!");
                return 0;
            }
        };

        og(this, sample_time, cmd)
    };

    info!("hk_create_move this={this:?} sample_time={sample_time} cmd={cmd:?} rc={rc}");

    rc
}
