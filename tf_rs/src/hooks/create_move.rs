use std::ffi::c_void;

use crate::{
    config::Config,
    features::{aimbot, backtrack, chams, fake_ping, movement, prediction_log, sticky},
    globals::Globals,
    helpers,
    hooks::Hooks,
    interfaces::Interfaces,
    types::UserCmd,
};

// Thirdperson toggle state
static mut TP: i32 = 0;
static mut LAST_TP: bool = false;

/// Put the local player back in firstperson if we left them in thirdperson.
pub fn reset_thirdperson() {
    unsafe {
        if TP == 0 {
            return;
        }
        TP = 0;
    }
    if !Interfaces::engine_client().is_in_game() {
        return;
    }
    if let Some(localplayer) = helpers::get_localplayer() {
        localplayer.set_thirdperson(0);
    }
}

pub extern "C" fn hk_create_move(this: *mut c_void, sample_time: f32, cmd: *mut UserCmd) -> i64 {
    let rc = Hooks::create_move()
        .call_create_move(this, sample_time, cmd)
        .expect("Invalid CreateMove function signature");

    if !Interfaces::engine_client().is_in_game() {
        return rc;
    }

    chams::init();
    let backtrack_config = Config::read().backtrack.clone();
    fake_ping::update(&backtrack_config);

    let localplayer = helpers::get_localplayer().expect("Failed to get localplayer");

    if localplayer.is_dead() {
        return rc;
    }

    // A copy, so no lock is held while calling into the game
    let config = Config::read().clone();

    movement::bunnyhop(&localplayer, cmd, &config);
    let choice = aimbot::run(&localplayer, cmd, &config);
    if config.aimbot.master
        && config.aimbot.projectile
        && config.aimbot.auto_detonate
        && !Globals::read().is_spectated
    {
        sticky::run(&localplayer, unsafe { &mut *cmd }, &config);
    }
    backtrack::run(&localplayer, cmd, &config, choice);
    if config.aimbot.projectile && config.aimbot.log_prediction {
        prediction_log::sample(&localplayer, &config);
    }

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
