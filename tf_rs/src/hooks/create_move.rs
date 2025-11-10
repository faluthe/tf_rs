use std::ffi::c_void;

use crate::{
    features::{aimbot, movement},
    helpers,
    hooks::Hooks,
    interfaces::Interfaces,
    types::UserCmd,
};

pub extern "C" fn hk_create_move(this: *mut c_void, sample_time: f32, cmd: *mut UserCmd) -> i64 {
    let rc = Hooks::create_move()
        .original
        .call_create_move(this, sample_time, cmd)
        .expect("Invalid CreateMove function signature");

    if !Interfaces::engine_client().is_in_game() {
        return rc;
    }

    let localplayer = helpers::get_localplayer().expect("Failed to get localplayer");

    movement::bunnyhop(&localplayer, cmd);
    aimbot::run(&localplayer, cmd);

    rc
}
