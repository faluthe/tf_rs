//! Movement prediction accuracy logging.
//!
//! Every so often each enemy is simulated a second ahead, with and without strafe prediction and
//! as a straight line. Every update the server sends for them after that is compared with where
//! the prediction had them at that simulation time, and a row per comparison goes to
//! prediction.csv next to the library. Errors at a few ticks either side are checked too, a best
//! offset away from zero means the prediction runs early or late rather than going the wrong way.

use std::{
    ffi::OsStr,
    io::Write,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use once_cell::sync::Lazy;

use crate::{
    config::Config,
    features::movesim::{self, MoveSim, Strafe},
    helpers,
    interfaces::Interfaces,
    types::{Player, Vec3},
};

const TICKS: usize = 66;
const SAMPLE_INTERVAL: f32 = 0.3;
const OFFSETS: i32 = 4;
static NEXT_ID: AtomicU32 = AtomicU32::new(0);
const FILE: &str = "prediction.csv";
const HEADER: &str = "id,variant,player,class,k,err_xy,err_z,best_offset,best_err,start_ground,ground,speed,avg_yaw,choke,delta\n";

struct Pending {
    id: u32,
    player: i32,
    variant: &'static str,
    /// Simulation time `positions[0]` is at, each next one is a tick later
    start_time: f32,
    positions: Vec<Vec3>,
    class: i32,
    start_ground: bool,
    speed: f32,
    average_yaw: f32,
    choke: i32,
    delta: f32,
    last_seen: f32,
}

#[derive(Default)]
struct State {
    pending: Vec<Pending>,
    last_sample: Vec<f32>,
    rows: String,
    last_flush: f32,
}

static STATE: Lazy<Mutex<State>> = Lazy::new(|| {
    Mutex::new(State {
        last_sample: vec![f32::MIN; 65],
        ..Default::default()
    })
});

fn simulate(player: &Player, strafe: Strafe) -> Option<(Vec<Vec3>, f32, i32, f32)> {
    let mut sim = MoveSim::new(player, None, strafe)?;
    while sim.path.len() < TICKS {
        sim.run_tick();
    }
    let mut positions = sim.path.clone();
    positions.push(sim.origin());
    Some((
        positions,
        sim.average_yaw(),
        sim.choke(),
        sim.predicted_delta(),
    ))
}

/// Start new predictions for enemies due one, from CreateMove
pub fn sample(localplayer: &Player, config: &Config) {
    let now = Interfaces::global_vars().realtime;
    let tick = Interfaces::global_vars().interval_per_tick;
    let strafe = Strafe::from(&config.aimbot);
    let last_sample = STATE.lock().unwrap().last_sample.clone();
    let due = helpers::enemies(localplayer, false).filter(|(i, _)| {
        last_sample
            .get(*i as usize)
            .is_some_and(|t| now - t >= SAMPLE_INTERVAL)
    });

    let mut new = Vec::new();
    for (i, player) in due {
        let start_time = player.simulation_time();
        let start_ground = player.is_on_ground();
        let velocity = player.velocity();
        let origin = player.network_origin();

        let full = simulate(&player, strafe);
        let plain = simulate(&player, Strafe::NONE);
        let gravity = movesim::gravity();
        let linear: Vec<Vec3> = (0..=TICKS)
            .map(|j| {
                let t = j as f32 * tick;
                let fall = if start_ground {
                    0.0
                } else {
                    0.5 * gravity * t * t
                };
                origin + velocity * t - Vec3::new(0.0, 0.0, fall)
            })
            .collect();

        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let mut push = |variant, positions, average_yaw, choke, delta| {
            new.push(Pending {
                id,
                player: i,
                variant,
                start_time,
                positions,
                class: player.player_class() as i32,
                start_ground,
                speed: velocity.length_2d(),
                average_yaw,
                choke,
                delta,
                last_seen: start_time,
            })
        };
        let (mut choke, mut delta) = (0, tick);
        if let Some((positions, yaw, c, d)) = full {
            (choke, delta) = (c, d);
            push("sim", positions, yaw, c, d);
        }
        if let Some((positions, yaw, c, d)) = plain {
            push("nostrafe", positions, yaw, c, d);
        }
        push("linear", linear, 0.0, choke, delta);
    }

    let mut state = STATE.lock().unwrap();
    for p in new {
        state.last_sample[p.player as usize] = now;
        state.pending.push(p);
    }
}

/// Compare pending predictions with the updates that just arrived, from FrameStageNotify
pub fn observe() {
    let tick = Interfaces::global_vars().interval_per_tick;
    let now = Interfaces::global_vars().realtime;
    let mut state = STATE.lock().unwrap();
    let mut rows = String::new();

    state.pending.retain_mut(|p| {
        let Some(player) = Interfaces::entity_list().get_client_entity::<Player>(p.player) else {
            return false;
        };
        if player.is_dormant() || player.is_dead() {
            return false;
        }
        let sim_time = player.simulation_time();
        if sim_time <= p.last_seen {
            return true;
        }
        p.last_seen = sim_time;
        let k = ((sim_time - p.start_time) / tick).round() as i32;
        if k as usize > TICKS {
            return false;
        }
        let actual = player.network_origin();
        let predicted = p.positions[k as usize];
        let err_xy = (actual - predicted).length_2d();
        let err_z = actual.z - predicted.z;
        let (best_offset, best_err) = (-OFFSETS..=OFFSETS)
            .filter_map(|o| {
                let j = k + o;
                (0..=TICKS as i32)
                    .contains(&j)
                    .then(|| (o, actual.distance_to(&p.positions[j as usize])))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap_or((0, 0.0));
        rows.push_str(&format!(
            "{},{},{},{},{},{:.2},{:.2},{},{:.2},{},{},{:.1},{:.3},{},{:.4}\n",
            p.id,
            p.variant,
            p.player,
            p.class,
            k,
            err_xy,
            err_z,
            best_offset,
            best_err,
            p.start_ground as i32,
            player.is_on_ground() as i32,
            p.speed,
            p.average_yaw,
            p.choke,
            p.delta,
        ));
        true
    });

    state.rows.push_str(&rows);
    if !state.rows.is_empty() && now - state.last_flush >= 1.0 {
        state.last_flush = now;
        let rows = std::mem::take(&mut state.rows);
        drop(state);
        write_rows(&rows);
    }
}

fn path() -> Option<PathBuf> {
    let lib = helpers::lib_path()?;
    Some(
        Path::new(OsStr::from_bytes(lib.as_bytes()))
            .parent()?
            .join(FILE),
    )
}

fn write_rows(rows: &str) {
    let Some(path) = path() else {
        return;
    };
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path);
    match file {
        Ok(mut f) => {
            let empty = f.metadata().is_ok_and(|m| m.len() == 0);
            let _ = if empty {
                f.write_all(HEADER.as_bytes())
            } else {
                Ok(())
            }
            .and_then(|_| f.write_all(rows.as_bytes()));
        }
        Err(e) => log::warn!("Could not write {}: {e}", path.display()),
    }
}
