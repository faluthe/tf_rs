use std::ffi::c_void;

use crate::{features::backtrack, hooks::Hooks, interfaces::Interfaces};

const FRAME_NET_UPDATE_END: i32 = 4;

pub extern "C" fn hk_frame_stage_notify(this: *mut c_void, stage: i32) {
    Hooks::frame_stage_notify()
        .call_frame_stage_notify(this, stage)
        .expect("Invalid FrameStageNotify function signature");

    // Entities now hold what the server just sent, before interpolation moves them back
    if stage == FRAME_NET_UPDATE_END && Interfaces::engine_client().is_in_game() {
        backtrack::update_records();
    }
}
