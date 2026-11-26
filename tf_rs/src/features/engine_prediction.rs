//! Engine prediction: run the command being built through the game's movement on the local player
//! before the aimbots, so they see us where the server will have us when it fires our weapon.
//!
//! The server runs a command's movement before its weapon code, so shots leave from where that
//! movement puts us, not from where the last command left us. While a `Prediction` is alive the
//! local player's origin, velocity and everything movement touches (ducking, view offset, ground)
//! are as they'll be after this command. Dropping it puts them back.

use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use crate::{
    interfaces::Interfaces,
    types::{MoveData, Player, UserCmd, Vec3, datamap::Snapshot},
};

/// SetupMove reads the last command's angles back out of the move data, so it's kept between runs
static MOVE_DATA: Mutex<Option<MoveData>> = Mutex::new(None);

static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Whether the local player has already been moved by the command being built
pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// The local player moved forward by the command being built, until this is dropped
pub struct Prediction {
    player: Player,
    snapshot: Snapshot,
    origins: (Vec3, Vec3),
    velocity: Vec3,
}

impl Prediction {
    pub fn start(localplayer: &Player, cmd: *mut UserCmd) -> Option<Self> {
        let helper = Interfaces::move_helper();
        if localplayer.is_dead() || !helper.is_valid() || cmd.is_null() {
            return None;
        }
        let snapshot = Snapshot::take(localplayer.this, localplayer.pred_desc_map())?;
        let player = Player {
            ent: localplayer.ent,
        };
        let prediction = Prediction {
            player: Player { ent: player.ent },
            snapshot,
            origins: (player.origin(), player.network_origin()),
            velocity: player.velocity(),
        };

        let game = Interfaces::prediction();
        let globals = Interfaces::global_vars_mut();
        let old_globals = unsafe { *globals };
        let (in_prediction, first_time) = (game.in_prediction(), game.first_time_predicted());

        unsafe {
            (*globals).tickcount = player.tick_base();
            (*globals).curtime = player.curtime();
            (*globals).frametime = if game.engine_paused() {
                0.0
            } else {
                (*globals).interval_per_tick
            };
        }
        // Not the first time, so no sounds or effects
        game.set_in_prediction(true);
        game.set_first_time_predicted(false);
        helper.set_host(player.this);
        player.set_current_command(cmd as *mut _);

        let mut stored = MOVE_DATA.lock().unwrap();
        let data = stored.get_or_insert_with(MoveData::default);
        game.setup_move(player.this, cmd, helper.this(), data);
        Interfaces::game_movement().process_movement(player.this, data);

        player.set_current_command(std::ptr::null_mut());
        helper.set_host(std::ptr::null_mut());
        game.set_in_prediction(in_prediction);
        game.set_first_time_predicted(first_time);
        unsafe {
            (*globals).tickcount = old_globals.tickcount;
            (*globals).curtime = old_globals.curtime;
            (*globals).frametime = old_globals.frametime;
        }

        // Where FinishMove would put us. Written straight to the fields we read, SetLocalOrigin
        // only marks the absolute origin dirty and ours are raw reads.
        player.set_origin(data.abs_origin);
        player.set_network_origin(data.abs_origin);
        player.set_velocity(data.velocity);
        ACTIVE.store(true, Ordering::Relaxed);
        Some(prediction)
    }
}

impl Drop for Prediction {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Relaxed);
        self.snapshot.restore();
        self.player.set_origin(self.origins.0);
        self.player.set_network_origin(self.origins.1);
        self.player.set_velocity(self.velocity);
    }
}
