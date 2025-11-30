use std::ffi::c_void;

use log::info;

use crate::{
    config::Config,
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
    info!("localplayer.this {:p}", localplayer.this);
    let config = Config::read();

    movement::bunnyhop(&localplayer, cmd, &config);
    aimbot::run(&localplayer, cmd, &config);

    if config.aimbot.silent_aim != 0 { 0 } else { rc }
}
