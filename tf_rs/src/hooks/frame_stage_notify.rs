use std::ffi::c_void;

use crate::{
    config::Config,
    features::{backtrack, chams, glow, movesim, pickups, prediction_log, projectile, shot_log},
    hooks::Hooks,
    interfaces::Interfaces,
};

const FRAME_START: i32 = 0;
const FRAME_NET_UPDATE_POSTDATAUPDATE_END: i32 = 3;
const FRAME_NET_UPDATE_END: i32 = 4;
const FRAME_RENDER_START: i32 = 5;

pub extern "C" fn hk_frame_stage_notify(this: *mut c_void, stage: i32) {
    Hooks::frame_stage_notify()
        .call_frame_stage_notify(this, stage)
        .expect("Invalid FrameStageNotify function signature");

    // Outside a map too, the menu preview draws chams before any map is joined
    if stage == FRAME_START {
        chams::init();
        glow::init();
    }
    if !Interfaces::engine_client().is_in_game() {
        return;
    }
    match stage {
        // Interpolation was just set up for the new data, undo it before it runs
        FRAME_NET_UPDATE_POSTDATAUPDATE_END => backtrack::disable_interpolation(),
        // Entities now hold what the server just sent, before interpolation moves them back
        FRAME_NET_UPDATE_END => {
            projectile::forget_projectiles();
            pickups::update();
            backtrack::update_records();
            let (simulating, logging) = {
                let aim = &Config::read().aimbot;
                let logging = aim.projectile && aim.log_prediction;
                (
                    aim.master && (aim.melee || aim.projectile) || logging,
                    logging,
                )
            };
            if simulating {
                movesim::update();
            }
            if logging {
                prediction_log::observe();
                shot_log::observe();
            }
        }
        FRAME_RENDER_START => backtrack::start_frame(),
        _ => {}
    }
}
