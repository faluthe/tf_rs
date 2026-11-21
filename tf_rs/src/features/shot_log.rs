//! Projectile shot logging, to see where the projectile aimbot goes wrong.
//!
//! - no_shot.csv: while the aimbot is looking and finds nothing, why each target was turned down
//! - shots.csv: each shot fired, with the timeline it assumed against the server time it fires at
//! - shot_projectile.csv: the projectile that shot made, against the path it was predicted to take
//! - shot_target.csv: where the target really was when the shot was predicted to land
//!
//! Everything is lined up by server time. The shot fires at our tick base, our projectiles and the
//! target carry the simulation time the server had them at.

use std::{
    collections::HashSet, ffi::OsStr, io::Write, os::unix::ffi::OsStrExt, path::Path, sync::Mutex,
};

use once_cell::sync::Lazy;

use crate::{
    features::projectile::Reject,
    helpers,
    interfaces::Interfaces,
    types::{ClassId, Entity, Player, Vec3},
};

const OWNER: usize = 0x754;
const THROWER: usize = 0xC30;
const OFFSETS: i32 = 4;
/// Seconds a shot waits for its projectile to show up
const SPAWN_WINDOW: f32 = 1.0;
/// How far off its shot's path a new projectile can be and still be matched to it
const MATCH_DISTANCE: f32 = 150.0;

/// What a shot was worked out from
#[derive(Clone, Copy, Default)]
pub struct Prediction {
    pub target: i32,
    /// Simulation time of the target's last update, the sim starts there
    pub start_time: f32,
    /// Ticks the sim ran to catch up to now
    pub choke: i32,
    /// Ticks the sim ran for our latency
    pub latency_ticks: i32,
    /// Ticks after that until the projectile lands
    pub ticks: i32,
    /// Predicted origin of the target at that point
    pub origin: Vec3,
    pub point: Vec3,
    pub splash: bool,
    pub target_on_ground: bool,
}

#[derive(Default)]
pub struct TargetDiagnostics {
    pub index: i32,
    pub fov: f32,
    pub distance: f32,
    pub on_ground: bool,
    pub no_sim: bool,
    /// Every shot would land after one of ours already on its way
    pub waiting: bool,
    pub late: u32,
    pub bad: u32,
    pub out_of_fov: u32,
    pub blocked: u32,
}

impl TargetDiagnostics {
    pub fn count(&mut self, reject: Reject) {
        match reject {
            Reject::Late => self.late += 1,
            Reject::Bad => self.bad += 1,
            Reject::Fov => self.out_of_fov += 1,
            Reject::Blocked => self.blocked += 1,
        }
    }
}

struct Shot {
    id: u32,
    /// Server time the projectile leaves, as the aimbot worked it out
    time: f32,
    prediction: Prediction,
    /// Server time it was predicted to land at
    land_time: f32,
    path: Vec<Vec3>,
    created: f32,
    projectile: Option<u32>,
    last_projectile_time: f32,
    min_distance: f32,
    target_logged: bool,
}

#[derive(Default)]
struct State {
    next_id: u32,
    shots: Vec<Shot>,
    /// Our projectiles already around, so only new ones get matched to shots
    known: HashSet<u32>,
    last_no_shot: f32,
    files: [String; 4],
    last_flush: f32,
}

static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::default()));

const NO_SHOT: usize = 0;
const SHOTS: usize = 1;
const PROJECTILE: usize = 2;
const TARGET: usize = 3;
const FILES: [(&str, &str); 4] = [
    (
        "no_shot.csv",
        "time,target,fov,distance,on_ground,no_sim,waiting,late,bad,out_of_fov,blocked\n",
    ),
    (
        "shots.csv",
        "id,target,class,target_on_ground,distance,fov,flight_time,ticks,latency_ticks,choke,timeline_error,splash,point_dz,point_dxy\n",
    ),
    (
        "shot_projectile.csv",
        "id,k,path_len,err,best_offset,best_err,target_distance\n",
    ),
    (
        "shot_target.csv",
        "id,late_by,err_xy,err_z,on_ground,predicted_on_ground,min_distance\n",
    ),
];

fn tick() -> f32 {
    Interfaces::global_vars().interval_per_tick
}

/// The aimbot looked and found nothing
pub fn no_shot(diagnostics: &[TargetDiagnostics]) {
    let now = Interfaces::global_vars().realtime;
    let mut state = STATE.lock().unwrap();
    // Every frame would be a lot of rows saying the same thing
    if now - state.last_no_shot < 0.1 {
        return;
    }
    state.last_no_shot = now;
    for d in diagnostics {
        state.files[NO_SHOT].push_str(&format!(
            "{:.2},{},{:.1},{:.0},{},{},{},{},{},{},{}\n",
            now,
            d.index,
            d.fov,
            d.distance,
            d.on_ground as i32,
            d.no_sim as i32,
            d.waiting as i32,
            d.late,
            d.bad,
            d.out_of_fov,
            d.blocked,
        ));
    }
}

/// A shot is being fired on this command
pub fn shot(localplayer: &Player, p: &Prediction, path: &[Vec3], flight_time: f32, fov: f32) {
    let time = localplayer.curtime();
    let tick = tick();
    // Where the sim put the projectile leaving against when the server will really fire it
    let assumed = p.start_time + (p.choke + p.latency_ticks) as f32 * tick;
    let target = Interfaces::entity_list().get_client_entity::<Player>(p.target);
    let class = target
        .as_ref()
        .map_or(0, |t| unsafe { *t.ent.field::<i32>(0x1BA0) });
    let distance = target
        .as_ref()
        .map_or(0.0, |t| t.origin().distance_to(&localplayer.eye_pos()));

    let mut state = STATE.lock().unwrap();
    let id = state.next_id;
    state.next_id += 1;
    state.files[SHOTS].push_str(&format!(
        "{},{},{},{},{:.0},{:.2},{:.3},{},{},{},{:.3},{},{:.1},{:.1}\n",
        id,
        p.target,
        class,
        p.target_on_ground as i32,
        distance,
        fov,
        flight_time,
        p.ticks,
        p.latency_ticks,
        p.choke,
        assumed - time,
        p.splash as i32,
        p.point.z - p.origin.z,
        (p.point - p.origin).length_2d(),
    ));
    state.shots.push(Shot {
        id,
        time: assumed,
        prediction: *p,
        land_time: assumed + p.ticks as f32 * tick,
        path: path.to_vec(),
        created: Interfaces::global_vars().realtime,
        projectile: None,
        last_projectile_time: 0.0,
        min_distance: f32::MAX,
        target_logged: false,
    });
}

fn is_projectile(ent: &Entity) -> bool {
    matches!(
        ent.class_id(),
        Some(
            ClassId::Rocket
                | ClassId::PillOrSticky
                | ClassId::Arrow
                | ClassId::Flare
                | ClassId::CrossbowBolt
        )
    )
}

fn nearest_distance(point: &Vec3, player: &Player) -> f32 {
    let (o, mins, maxs) = (player.network_origin(), player.mins(), player.maxs());
    let nearest = Vec3::new(
        point.x.clamp(o.x + mins.x, o.x + maxs.x),
        point.y.clamp(o.y + mins.y, o.y + maxs.y),
        point.z.clamp(o.z + mins.z, o.z + maxs.z),
    );
    point.distance_to(&nearest)
}

/// Match new projectiles to shots and compare what came in with the predictions, every update
pub fn observe() {
    let Some(localplayer) = helpers::get_localplayer() else {
        return;
    };
    let local = localplayer.index();
    let list = Interfaces::entity_list();
    let tick = tick();
    let now = Interfaces::global_vars().realtime;

    // Our projectiles out right now
    let mut ours: Vec<(u32, Entity)> = Vec::new();
    for i in 1..=list.max_entities() {
        let Some(ent) = list.get_client_entity::<Entity>(i) else {
            continue;
        };
        if ent.is_dormant() || !is_projectile(&ent) {
            continue;
        }
        let owner = (unsafe { *ent.field::<u32>(OWNER) }) & 0xFFF;
        let thrower = if matches!(ent.class_id(), Some(ClassId::PillOrSticky)) {
            (unsafe { *ent.field::<u32>(THROWER) }) & 0xFFF
        } else {
            u32::MAX
        };
        if owner as i32 == local || thrower as i32 == local {
            ours.push((ent.ref_handle(), ent));
        }
    }

    let mut state = STATE.lock().unwrap();
    let mut rows = [String::new(), String::new()];

    // A new one belongs to the waiting shot whose predicted path it's on
    for (handle, ent) in &ours {
        if state.known.contains(handle) {
            continue;
        }
        let (sim_time, actual) = (ent.simulation_time(), ent.network_origin());
        let closest = state
            .shots
            .iter_mut()
            .filter(|s| s.projectile.is_none() && now - s.created < SPAWN_WINDOW)
            .map(|s| {
                let k = ((sim_time - s.time) / tick).round() as i32;
                let j = k.clamp(0, s.path.len() as i32 - 1) as usize;
                (actual.distance_to(&s.path[j]), s)
            })
            .filter(|(d, _)| *d < MATCH_DISTANCE)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, shot)) = closest {
            shot.projectile = Some(*handle);
        }
    }
    state.known = ours.iter().map(|(h, _)| *h).collect();

    state.shots.retain_mut(|shot| {
        let target = list.get_client_entity::<Player>(shot.prediction.target);
        let projectile = shot
            .projectile
            .and_then(|h| ours.iter().find(|(o, _)| *o == h).map(|(_, e)| e));

        if let Some(ent) = projectile {
            let sim_time = ent.simulation_time();
            if sim_time > shot.last_projectile_time {
                shot.last_projectile_time = sim_time;
                let k = ((sim_time - shot.time) / tick).round() as i32;
                let actual = ent.network_origin();
                let at = |j: i32| shot.path[j.clamp(0, shot.path.len() as i32 - 1) as usize];
                let err = actual.distance_to(&at(k));
                let (best_offset, best_err) = (-OFFSETS..=OFFSETS)
                    .map(|o| (o, actual.distance_to(&at(k + o))))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .unwrap_or((0, err));
                let target_distance = target
                    .as_ref()
                    .filter(|t| !t.is_dead() && !t.is_dormant())
                    .map_or(-1.0, |t| nearest_distance(&actual, t));
                if target_distance >= 0.0 {
                    shot.min_distance = shot.min_distance.min(target_distance);
                }
                rows[0].push_str(&format!(
                    "{},{},{},{:.1},{},{:.1},{:.1}\n",
                    shot.id,
                    k,
                    shot.path.len(),
                    err,
                    best_offset,
                    best_err,
                    target_distance,
                ));
            }
        }

        if !shot.target_logged {
            if let Some(t) = target.as_ref().filter(|t| !t.is_dead() && !t.is_dormant()) {
                let sim_time = t.simulation_time();
                if sim_time >= shot.land_time - tick * 0.5 {
                    shot.target_logged = true;
                    let actual = t.network_origin();
                    let d = actual - shot.prediction.origin;
                    rows[1].push_str(&format!(
                        "{},{:.3},{:.1},{:.1},{},{},{:.1}\n",
                        shot.id,
                        sim_time - shot.land_time,
                        d.length_2d(),
                        d.z,
                        t.is_on_ground() as i32,
                        shot.prediction.target_on_ground as i32,
                        if shot.min_distance == f32::MAX {
                            -1.0
                        } else {
                            shot.min_distance
                        },
                    ));
                }
            } else {
                shot.target_logged = true;
            }
        }

        // Done once the projectile is gone and the target's been checked, or it never showed
        let waiting = match shot.projectile {
            None => now - shot.created < SPAWN_WINDOW,
            Some(_) => projectile.is_some(),
        };
        waiting || !shot.target_logged
    });

    let [projectile_rows, target_rows] = rows;
    state.files[PROJECTILE].push_str(&projectile_rows);
    state.files[TARGET].push_str(&target_rows);

    if now - state.last_flush >= 1.0 {
        state.last_flush = now;
        let files = std::mem::take(&mut state.files);
        drop(state);
        for (i, rows) in files.iter().enumerate() {
            if !rows.is_empty() {
                write_rows(FILES[i].0, FILES[i].1, rows);
            }
        }
    }
}

fn write_rows(name: &str, header: &str, rows: &str) {
    let Some(lib) = helpers::lib_path() else {
        return;
    };
    let Some(dir) = Path::new(OsStr::from_bytes(lib.as_bytes())).parent() else {
        return;
    };
    let path = dir.join(name);
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut f) => {
            let empty = f.metadata().is_ok_and(|m| m.len() == 0);
            let header = if empty { header } else { "" };
            let _ = f.write_all(format!("{header}{rows}").as_bytes());
        }
        Err(e) => log::warn!("Could not write {}: {e}", path.display()),
    }
}
