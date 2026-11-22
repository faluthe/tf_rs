//! Projectile shot logging, to see where the projectile aimbot goes wrong.
//!
//! - no_shot.csv: while the aimbot is looking and finds nothing, why each target was turned down
//! - shots.csv: each shot fired, with the timeline it assumed against the server time it fires at
//! - shot_projectile.csv: the projectile that shot made, against the path it was predicted to take
//! - shot_target.csv: where the target really was when the shot was predicted to land
//! - shot_launch.csv: where each shot's projectile was predicted to start and how fast
//! - shot_flight.csv: where each shot's projectile really was every update, by ticks since firing
//! - shot_miss.csv: how close each projectile came to its target and which way it missed, and how
//!   many ticks earlier or later the target would have had to be for it to come closest
//! - perf.csv: each second, commands and frames, and how long the aimbot's search took
//!
//! Everything is lined up by server time. The shot fires at our tick base, our projectiles and the
//! target carry the simulation time the server had them at.

use std::{collections::HashSet, sync::Mutex};

use once_cell::sync::Lazy;

use crate::{
    features::projectile::{self, Reject},
    helpers,
    interfaces::Interfaces,
    types::{Hull, Player, Vec3},
};

const OFFSETS: i32 = 4;
/// Seconds a shot waits for its projectile to show up
const SPAWN_WINDOW: f32 = 1.0;
/// How far off its shot's path a new projectile can be and still be matched to it
const MATCH_DISTANCE: f32 = 150.0;
/// Ticks either way the target's track is shifted by to find when the projectile would have hit
const SHIFTS: i32 = 10;

/// What a shot was worked out from
#[derive(Clone, Copy, Default)]
pub struct Prediction {
    pub target: i32,
    /// Simulation time of the target's last update, the sim starts there
    pub start_time: f32,
    /// Ticks the sim ran to catch up to now
    pub choke: i32,
    /// Ticks the sim ran after catching up, until the shot leaves at our tick base
    pub latency_ticks: i32,
    /// Ticks after that until the projectile lands
    pub ticks: i32,
    /// Predicted origin of the target at that point
    pub origin: Vec3,
    pub point: Vec3,
    pub splash: bool,
    pub target_on_ground: bool,
    /// Predicted start position and velocity of the projectile
    pub launch: (Vec3, Vec3),
    pub gravity: f32,
    pub drag: f32,
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
    pub blocked: u32,
    pub miss: u32,
}

impl TargetDiagnostics {
    pub fn count(&mut self, reject: Reject) {
        match reject {
            Reject::Late => self.late += 1,
            Reject::Bad => self.bad += 1,
            Reject::Blocked => self.blocked += 1,
            Reject::Miss => self.miss += 1,
        }
    }
}

struct Shot {
    id: u32,
    /// Our tick base when the projectile leaves, the clock it flies on
    time: f32,
    prediction: Prediction,
    /// When it was predicted to land, on the target's clock
    land_time: f32,
    path: Vec<Vec3>,
    created: f32,
    projectile: Option<u32>,
    last_projectile_time: f32,
    min_distance: f32,
    target_logged: bool,
    weapon: i32,
    local_on_ground: bool,
    local_velocity: Vec3,
    /// How far the projectile was off its path when it first showed up, and at which step
    spawn: Option<(i32, f32)>,
    /// Projectile positions by server tick
    projectile_track: Vec<(i32, Vec3)>,
    /// Target origins by the tick of their simulation time, from the shot on
    target_track: Vec<(i32, Vec3)>,
    target_mins: Vec3,
    target_maxs: Vec3,
}

/// Search timing over the current second
#[derive(Default)]
struct Perf {
    start: f32,
    searches: u32,
    frames: u32,
    last_frame: i32,
    frame_time: f32,
    total_ms: f32,
    max_ms: f32,
}

#[derive(Default)]
struct State {
    next_id: u32,
    shots: Vec<Shot>,
    /// Our projectiles already around, so only new ones get matched to shots
    known: HashSet<u32>,
    last_no_shot: f32,
    perf: Perf,
    files: [String; 8],
    last_flush: f32,
}

static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::default()));

const NO_SHOT: usize = 0;
const SHOTS: usize = 1;
const PROJECTILE: usize = 2;
const TARGET: usize = 3;
const PERF: usize = 4;
const MISS: usize = 5;
const LAUNCH: usize = 6;
const FLIGHT: usize = 7;
const FILES: [(&str, &str); 8] = [
    (
        "no_shot.csv",
        "time,target,fov,distance,on_ground,no_sim,waiting,late,bad,blocked,miss\n",
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
    (
        "perf.csv",
        "time,searches,frames,searches_per_frame,fps,avg_ms,max_ms\n",
    ),
    (
        "shot_miss.csv",
        "id,weapon,local_on_ground,local_speed,local_vz,target_on_ground,flight_time,spawn_k,spawn_err,samples,min_dist,miss_along,miss_side,miss_z,target_speed,best_shift,best_min_dist\n",
    ),
    ("shot_launch.csv", "id,weapon,x,y,z,vx,vy,vz,gravity,drag\n"),
    ("shot_flight.csv", "id,k,x,y,z\n"),
];

/// One search took `ms`, from CreateMove
pub fn search_time(ms: f32) {
    let globals = Interfaces::global_vars();
    let mut state = STATE.lock().unwrap();
    let p = &mut state.perf;
    if p.searches == 0 {
        p.start = globals.realtime;
    }
    p.searches += 1;
    if globals.framecount != p.last_frame {
        p.last_frame = globals.framecount;
        p.frames += 1;
        p.frame_time += globals.absoluteframetime;
    }
    p.total_ms += ms;
    p.max_ms = p.max_ms.max(ms);
    if globals.realtime - p.start >= 1.0 {
        let p = std::mem::take(&mut state.perf);
        let frames = p.frames.max(1) as f32;
        state.files[PERF].push_str(&format!(
            "{:.1},{},{},{:.2},{:.0},{:.2},{:.2}\n",
            p.start,
            p.searches,
            p.frames,
            p.searches as f32 / frames,
            frames / p.frame_time.max(0.001),
            p.total_ms / p.searches as f32,
            p.max_ms,
        ));
    }
}

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
            d.blocked,
            d.miss,
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
    let class = target.as_ref().map_or(0, |t| t.player_class() as i32);
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
    let local_velocity = localplayer.velocity();
    let weapon = localplayer
        .active_weapon()
        .map_or(-1, |w| w.weapon_class() as i32);
    let (target_mins, target_maxs) = target
        .as_ref()
        .map_or((Vec3::default(), Vec3::default()), |t| (t.mins(), t.maxs()));
    let (lp, lv) = p.launch;
    state.files[LAUNCH].push_str(&format!(
        "{id},{weapon},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{},{}\n",
        lp.x, lp.y, lp.z, lv.x, lv.y, lv.z, p.gravity, p.drag
    ));
    state.shots.push(Shot {
        id,
        time,
        prediction: *p,
        land_time: assumed + p.ticks as f32 * tick,
        path: path.to_vec(),
        created: Interfaces::global_vars().realtime,
        projectile: None,
        last_projectile_time: 0.0,
        min_distance: f32::MAX,
        target_logged: false,
        weapon,
        local_on_ground: localplayer.is_on_ground(),
        local_velocity,
        spawn: None,
        projectile_track: Vec::new(),
        target_track: Vec::new(),
        target_mins,
        target_maxs,
    });
}

fn nearest_distance(point: &Vec3, player: &Player) -> f32 {
    let hull = Hull {
        origin: player.network_origin(),
        ..player.hull()
    };
    point.distance_to(&hull.nearest_point(point))
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

    let ours = projectile::our_projectiles(local);

    let mut state = STATE.lock().unwrap();
    let mut rows = [String::new(), String::new(), String::new(), String::new()];

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
                rows[3].push_str(&format!(
                    "{},{},{:.2},{:.2},{:.2}\n",
                    shot.id, k, actual.x, actual.y, actual.z
                ));
                if shot.spawn.is_none() {
                    shot.spawn = Some((k, err));
                }
                shot.projectile_track
                    .push(((sim_time / tick).round() as i32, actual));
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

        if let Some(t) = target.as_ref().filter(|t| !t.is_dead() && !t.is_dormant()) {
            let at = (t.simulation_time() / tick).round() as i32;
            if shot.target_track.last().is_none_or(|(last, _)| at > *last) {
                shot.target_track.push((at, t.network_origin()));
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
        let keep = waiting || !shot.target_logged;
        if !keep && shot.projectile.is_some() {
            rows[2].push_str(&miss_row(shot));
        }
        keep
    });

    let [projectile_rows, target_rows, miss_rows, flight_rows] = rows;
    state.files[FLIGHT].push_str(&flight_rows);
    state.files[PROJECTILE].push_str(&projectile_rows);
    state.files[TARGET].push_str(&target_rows);
    state.files[MISS].push_str(&miss_rows);

    if now - state.last_flush >= 1.0 {
        state.last_flush = now;
        let files = std::mem::take(&mut state.files);
        drop(state);
        for (i, rows) in files.iter().enumerate() {
            if !rows.is_empty() {
                helpers::append_csv(FILES[i].0, FILES[i].1, rows);
            }
        }
    }
}

/// Where the target was at `tick` on its own track, between the updates either side
fn target_at(track: &[(i32, Vec3)], tick: i32) -> Option<Vec3> {
    let i = track.partition_point(|(t, _)| *t < tick);
    let after = track.get(i)?;
    if after.0 == tick {
        return Some(after.1);
    }
    let before = track.get(i.checked_sub(1)?)?;
    let a = (tick - before.0) as f32 / (after.0 - before.0) as f32;
    Some(before.1 + (after.1 - before.1) * a)
}

/// Closest the projectile came to the target box with the target's track moved `shift` ticks
/// later, and where that was
fn closest(shot: &Shot, shift: i32) -> Option<(f32, Vec3, i32)> {
    shot.projectile_track
        .iter()
        .filter_map(|(tick, p)| {
            let o = target_at(&shot.target_track, tick + shift)?;
            let hull = Hull {
                origin: o,
                mins: shot.target_mins,
                maxs: shot.target_maxs,
            };
            Some((p.distance_to(&hull.nearest_point(p)), *p, *tick))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

/// Which way a finished shot missed its target
fn miss_row(shot: &Shot) -> String {
    let lv = shot.local_velocity;
    let (spawn_k, spawn_err) = shot.spawn.unwrap_or((-1, -1.0));
    let head = format!(
        "{},{},{},{:.0},{:.0},{},{:.3},{},{:.1},{}",
        shot.id,
        shot.weapon,
        shot.local_on_ground as i32,
        lv.length_2d(),
        lv.z,
        shot.prediction.target_on_ground as i32,
        shot.prediction.ticks as f32 * tick(),
        spawn_k,
        spawn_err,
        shot.projectile_track.len(),
    );
    let Some((dist, point, at)) = closest(shot, 0) else {
        return format!("{head},-1,0,0,0,0,0,-1\n");
    };
    // Miss from the middle of the box, split along the way the target was going
    let o = target_at(&shot.target_track, at).unwrap_or_default();
    let center = o + (shot.target_mins + shot.target_maxs) * 0.5;
    let miss = point - center;
    let velocity = match (
        target_at(&shot.target_track, at - 1),
        target_at(&shot.target_track, at + 1),
    ) {
        (Some(a), Some(b)) => (b - a) * (0.5 / tick()),
        _ => Vec3::default(),
    };
    let speed = velocity.length_2d();
    let (along, side) = if speed > 1.0 {
        let (fx, fy) = (velocity.x / speed, velocity.y / speed);
        (miss.x * fx + miss.y * fy, miss.y * fx - miss.x * fy)
    } else {
        (0.0, miss.length_2d())
    };
    let (best_shift, best) = (-SHIFTS..=SHIFTS)
        .filter_map(|s| closest(shot, s).map(|c| (s, c.0)))
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.abs().cmp(&b.0.abs())))
        .unwrap_or((0, dist));
    format!(
        "{head},{dist:.1},{along:.1},{side:.1},{:.1},{speed:.0},{best_shift},{best:.1}\n",
        miss.z
    )
}
