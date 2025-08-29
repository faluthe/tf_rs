use std::ffi::c_void;

use log::info;

use crate::{
    features::{helpers, movement::bunnyhop},
    hooks::Hooks,
};

pub extern "C" fn hk_create_move(this: *mut c_void, sample_time: f32, cmd: *mut c_void) -> i64 {
    let rc = Hooks::create_move()
        .original
        .call_create_move(this, sample_time, cmd)
        .expect("Invalid CreateMove function signature");

    // info!("hk_create_move this={this:?} sample_time={sample_time} cmd={cmd:?} rc={rc}");

    let localplayer = helpers::get_localplayer().expect("Failed to get localplayer");

    info!("Localplayer health: {}", localplayer.health());

    // bunnyhop(localplayer, cmd);

    rc
}
