use std::ffi::c_void;

use crate::{
    config::Config,
    features::{aimbot, movement},
    globals::Globals,
    helpers,
    hooks::Hooks,
    interfaces::Interfaces,
    types::UserCmd,
};

// Thirdperson toggle state
static mut TP: i32 = 0;
static mut LAST_TP: bool = false;

pub extern "C" fn hk_create_move(this: *mut c_void, sample_time: f32, cmd: *mut UserCmd) -> i64 {
    let rc = Hooks::create_move()
        .original
        .call_create_move(this, sample_time, cmd)
        .expect("Invalid CreateMove function signature");

    if !Interfaces::engine_client().is_in_game() {
        return rc;
    }

    let localplayer = helpers::get_localplayer().expect("Failed to get localplayer");

    if localplayer.is_dead() {
        return rc;
    }

    let config = Config::read();

    movement::bunnyhop(&localplayer, cmd, &config);
    aimbot::run(&localplayer, cmd, &config);

    let pressed = Globals::read().thirdperson_pressed;
    unsafe {
        if pressed && !LAST_TP {
            TP = if TP == 0 { 1 } else { 0 };
            localplayer.set_thirdperson(TP);
        }

        LAST_TP = pressed;
    }

    if config.aimbot.silent_aim != 0 { 0 } else { rc }
}
