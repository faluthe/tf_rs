use std::ffi::c_void;

use crate::{
    config::Config,
    features::{aimbot, chams, movement},
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
        .call_create_move(this, sample_time, cmd)
        .expect("Invalid CreateMove function signature");

    if !Interfaces::engine_client().is_in_game() {
        return rc;
    }

    chams::init();

    let localplayer = helpers::get_localplayer().expect("Failed to get localplayer");

    if localplayer.is_dead() {
        return rc;
    }

    let config = Config::read();

    movement::bunnyhop(&localplayer, cmd, &config);
    aimbot::run(&localplayer, cmd, &config);

    let pressed = Globals::read().thirdperson_pressed;
    unsafe {
        if !config.thirdperson.use_key {
            // Turned off from the menu while in thirdperson
            if TP != 0 {
                TP = 0;
                localplayer.set_thirdperson(TP);
            }
        } else if pressed && !LAST_TP {
            TP = if TP == 0 { 1 } else { 0 };
            localplayer.set_thirdperson(TP);
        }

        LAST_TP = pressed;
    }

    if config.aimbot.silent_aim { 0 } else { rc }
}
