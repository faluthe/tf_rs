//! Movement simulation: predict where a player will be by running the game's own movement code
//! on them, tick by tick, with the inputs they look to be pressing.
//!
//! Each network update of a player is recorded with the direction they're moving in. To simulate,
//! their predicted fields are saved, their movement is set up from the newest record and fed
//! through CTFGameMovement::ProcessMovement once per tick, and everything is restored after.
//! Strafe prediction turns the simulated view a little every tick by the average turn rate seen
//! over the recent records, so players circling or air strafing curve the way they have been.

use std::{
    collections::VecDeque,
    ffi::c_void,
    sync::{Mutex, OnceLock},
};

use once_cell::sync::{Lazy, OnceCell};

use crate::{
    config::AimbotConfig,
    helpers::{self, module},
    interfaces::{Interfaces, mask},
    types::{
        ConVar, Cond, Entity, MoveData, Player, TraceFilter, UserCmd, Vec3, datamap::Snapshot,
        entity::handle_index, player::FL_DUCKING, user_cmd::Buttons, vec3::normalize_angle,
    },
};

const MAX_RECORDS: usize = 66;
const MAX_DELTAS: usize = 5;
const MAX_ORIGINS: usize = 5;

/// Strafe prediction tuning: how many records are averaged and when a turn counts as steady
struct StrafeTuning {
    samples: usize,
    straight_fuzzy: f32,
    low_min_samples: f32,
    high_min_samples: f32,
    low_min_distance: f32,
    high_min_distance: f32,
    max_changes: i32,
    max_change_time: i32,
}

const GROUND: StrafeTuning = StrafeTuning {
    samples: 33,
    straight_fuzzy: 100.0,
    low_min_samples: 16.0,
    high_min_samples: 33.0,
    low_min_distance: 0.0,
    high_min_distance: 1000.0,
    max_changes: 0,
    max_change_time: 0,
};

const AIR: StrafeTuning = StrafeTuning {
    samples: 33,
    straight_fuzzy: 0.0,
    low_min_samples: 16.0,
    high_min_samples: 16.0,
    low_min_distance: 100000.0,
    high_min_distance: 100000.0,
    max_changes: 2,
    max_change_time: 16,
};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Ground,
    Air,
    Swim,
}

#[derive(Clone, Copy)]
struct MoveRecord {
    /// World direction of the inputs, scaled like forward/side move
    direction: Vec3,
    sim_time: f32,
    mode: Mode,
    velocity: Vec3,
    origin: Vec3,
}

#[derive(Default)]
struct History {
    /// Newest first
    records: VecDeque<MoveRecord>,
    /// Time between updates, newest first
    deltas: VecDeque<f32>,
    /// Top of the collision box at each update, for the velocity estimate
    origins: VecDeque<(Vec3, f32)>,
    sim_time: f32,
    update_tick: i32,
    avg_velocity: Option<Vec3>,
}

static HISTORY: Lazy<Mutex<Vec<History>>> =
    Lazy::new(|| Mutex::new((0..=64).map(|_| History::default()).collect()));

/// Which strafe predictions to make
#[derive(Clone, Copy)]
pub struct Strafe {
    pub ground: bool,
    pub air: bool,
}

impl Strafe {
    pub const NONE: Strafe = Strafe {
        ground: false,
        air: false,
    };
}

/// The ones turned on in the aimbot's settings
impl From<&AimbotConfig> for Strafe {
    fn from(aim: &AimbotConfig) -> Self {
        Strafe {
            ground: aim.strafe_ground,
            air: aim.strafe_air,
        }
    }
}

struct Cvars {
    gravity: Option<ConVar>,
    air_accelerate: Option<ConVar>,
}

fn cvars() -> &'static Cvars {
    static CVARS: OnceCell<Cvars> = OnceCell::new();
    CVARS.get_or_init(|| {
        let cvar = Interfaces::cvar();
        Cvars {
            gravity: cvar.find_var("sv_gravity"),
            air_accelerate: cvar.find_var("sv_airaccelerate"),
        }
    })
}

pub fn gravity() -> f32 {
    cvars().gravity.map_or(800.0, |c| c.float())
}

fn tick_interval() -> f32 {
    Interfaces::global_vars().interval_per_tick
}

fn time_to_ticks(t: f32) -> i32 {
    (0.5 + t / tick_interval()) as i32
}

/// Record each enemy's newest network update, run in FrameStageNotify NET_UPDATE_END
pub fn update() {
    let Some(localplayer) = helpers::get_localplayer() else {
        return;
    };
    let tickcount = Interfaces::global_vars().tickcount;
    let mut history = HISTORY.lock().unwrap();

    for i in 1..=Interfaces::engine_client().get_max_clients().min(64) {
        let h = &mut history[i as usize];
        let player = Interfaces::entity_list().get_client_entity::<Player>(i);
        let Some(player) =
            player.filter(|p| *p != localplayer && !p.is_dead() && p.team() != localplayer.team())
        else {
            *h = History::default();
            continue;
        };
        if player.is_dormant() {
            h.origins.clear();
            continue;
        }

        let sim_time = player.simulation_time();
        if sim_time <= h.sim_time {
            continue;
        }
        let first = h.sim_time == 0.0;
        let delta = sim_time - h.sim_time;
        h.sim_time = sim_time;
        h.update_tick = tickcount;
        if first {
            continue;
        }
        h.deltas
            .push_front(time_to_ticks(delta).clamp(1, 24) as f32 * tick_interval());
        h.deltas.truncate(MAX_DELTAS);

        record_origin(h, &player, sim_time);
        record_move(h, &player, sim_time);
    }
}

/// Velocity from the origins over the last few updates, where they agree with moving steadily.
/// Origins are networked to 1/8th of a unit and an update spans several ticks, so the velocity the
/// game estimates from the last two is noisy.
fn record_origin(h: &mut History, player: &Player, sim_time: f32) {
    let new = (
        player.origin() + Vec3::new(0.0, 0.0, player.maxs().z),
        sim_time,
    );
    let Some(&old) = h.origins.front() else {
        h.origins.push_front(new);
        return;
    };

    let ticks = time_to_ticks(new.1 - old.1);
    if ticks <= 0 {
        return;
    }
    let dt = ticks as f32 * tick_interval();
    // Teleported, sv_lagcompensation_teleport_dist is 64
    if (new.0 - old.0).length_2d().powi(2) >= 64.0 * 64.0 * ticks as f32 {
        h.origins.clear();
        h.origins.push_front(new);
        h.avg_velocity = None;
        return;
    }

    let grounded = player.is_on_ground();
    let v = [0, 1, 2].map(|i| axis_velocity(&h.origins, new, old, i, grounded, dt));
    h.avg_velocity = Some(Vec3::new(v[0], v[1], v[2]));

    h.origins.push_front(new);
    h.origins.truncate(MAX_ORIGINS);
}

/// Component `i` of `v`
fn component(v: &Vec3, i: usize) -> f32 {
    [v.x, v.y, v.z][i]
}

/// Like signum, but zero counts as positive either way
fn sign(v: f32) -> f32 {
    if v < 0.0 { -1.0 } else { 1.0 }
}

/// Lowest and highest velocity between two values of a coordinate networked `dt` apart, each
/// rounded towards zero to 1/8th of a unit
fn rounding_range(old: f32, new: f32, dt: f32) -> (f32, f32) {
    let bounds = |v: f32| {
        if v == 0.0 {
            (-0.125, 0.125)
        } else {
            (v, v + 0.125 * sign(v))
        }
    };
    let ((o1, o2), (n1, n2)) = (bounds(old), bounds(new));
    let deltas = [n1 - o1, n2 - o1, n1 - o2, n2 - o2];
    let min = deltas.iter().copied().fold(f32::MAX, f32::min);
    let max = deltas.iter().copied().fold(f32::MIN, f32::max);
    (min / dt, max / dt)
}

/// Velocity on axis `i` from the `new` origin back to the oldest of `origins` still in line with
/// moving steadily, by the range the newest two allow. `old` is the one before `new`, `dt` before
/// it, and gravity counts on the vertical axis unless `grounded`.
fn axis_velocity(
    origins: &VecDeque<(Vec3, f32)>,
    new: (Vec3, f32),
    old: (Vec3, f32),
    i: usize,
    grounded: bool,
    dt: f32,
) -> f32 {
    let (g, tick) = (gravity(), tick_interval());
    let (mut old_value, mut old_time) = (component(&old.0, i), old.1);
    let new_value = component(&new.0, i);
    if !(i == 2 && grounded) {
        let (min, max) = rounding_range(old_value, new_value, dt);
        // Go back while older origins fit that range
        for &(origin, time) in origins {
            if old_time <= time {
                continue;
            }
            let rewind = -((new.1 - time) / tick).round() * tick;
            let (mut lo, mut hi) = (new_value + max * rewind, new_value + min * rewind);
            if i == 2 {
                let correction = g * (rewind + tick / 2.0).powi(2) / 2.0;
                lo -= correction;
                hi -= correction;
            }
            let value = component(&origin, i);
            if lo > value || value > hi {
                break;
            }
            old_value = value;
            old_time = time;
        }
    }

    let ticks = time_to_ticks(new.1 - old_time).max(1);
    let mut correction = 0.0;
    if i == 2 && !grounded {
        let t = ticks as f32 + 0.5;
        correction = (t * t - t) / 2.0 * g * tick.powi(2);
    }
    let delta = new_value - old_value;
    let rounding = if delta != 0.0 {
        0.0625 * sign(new_value)
    } else {
        0.0
    };
    (delta + rounding - correction) / (ticks as f32 * tick)
}

fn record_move(h: &mut History, player: &Player, sim_time: f32) {
    let velocity = player.velocity();
    if velocity.is_zero() {
        h.records.clear();
        return;
    }
    let origin = player.network_origin();
    let mode = if player.is_swimming() {
        Mode::Swim
    } else if player.is_on_ground() {
        Mode::Ground
    } else {
        Mode::Air
    };

    // Ran into a wall, the turns before it say nothing about where they're going now
    if let Some(last) = h.records.front() {
        let shrink = Vec3::new(0.125, 0.125, 0.125);
        let trace = Interfaces::engine_trace().trace_hull(
            &last.origin,
            &(last.origin + last.velocity * tick_interval()),
            &(player.mins() + shrink),
            &(player.maxs() - shrink),
            mask::PLAYER_SOLID,
            &TraceFilter::world_only(),
        );
        if trace.did_hit() && trace.plane_normal().z < 0.707 {
            h.records.clear();
        }
    }

    let max_speed = player.max_speed();
    let direction = if player.in_cond(Cond::ShieldCharge) {
        // Charging moves straight forward at full speed
        let mut cmd = Movement {
            forward: 450.0,
            side: 0.0,
            up: 0.0,
        };
        cmd.fix(
            &Vec3::new(player.eye_pitch(), player.eye_yaw(), 0.0),
            &Vec3::default(),
        );
        Vec3::new(cmd.forward, -cmd.side, 0.0)
    } else {
        match mode {
            Mode::Ground => velocity.to_2d(),
            Mode::Air => velocity.to_2d().normalized() * max_speed,
            Mode::Swim => velocity.to_2d() * 2.0,
        }
    };

    h.records.push_front(MoveRecord {
        direction,
        sim_time,
        mode,
        velocity,
        origin,
    });
    h.records.truncate(MAX_RECORDS);
}

/// Forward, side and up move
#[derive(Clone, Copy, Default)]
struct Movement {
    forward: f32,
    side: f32,
    up: f32,
}

impl Movement {
    /// Re-express movement made relative to `from` as the same world movement relative to `to`
    fn fix(&mut self, from: &Vec3, to: &Vec3) {
        let from_oob = normalize_angle(from.x).abs() > 90.0;
        let to_oob = normalize_angle(to.x).abs() > 90.0;
        let side = self.side * if from_oob { -1.0 } else { 1.0 };
        let speed = (self.forward * self.forward + side * side).sqrt();
        let move_yaw = side.atan2(self.forward).to_degrees();
        let from_yaw = from.y + if from_oob { 180.0 } else { 0.0 };
        let to_yaw = to.y + if to_oob { 180.0 } else { 0.0 };
        let yaw = (to_yaw - from_yaw + move_yaw).to_radians();
        self.forward = yaw.cos() * speed;
        self.side = yaw.sin() * speed * if to_oob { -1.0 } else { 1.0 };
    }
}

/// How much of the averaged turn to keep in the air, less while moving up slowly. An estimate,
/// the game's air movement has no such term.
fn friction_scale(velocity_xy: f32, turn: f32, velocity_z: f32, min: f32, max: f32) -> f32 {
    if velocity_z <= 0.0 || velocity_z > 250.0 {
        return 1.0;
    }
    let scale = cvars().air_accelerate.map_or(10.0, |c| c.float()).max(1.0);
    let (min, max) = (min * scale, max * scale);
    let x = (velocity_xy * turn).abs();
    // Remap min..max onto 1..0.25, unclamped like Math::RemapVal
    if max == min {
        return 1.0;
    }
    1.0 + (x - min) / (max - min) * (0.25 - 1.0)
}

/// Turn between two records per tick, scaled by how much of the turn the speed allows
#[derive(Default)]
struct YawTracker {
    changes: i32,
    start: i32,
    sign: i32,
    zero: bool,
}

impl YawTracker {
    fn difference(
        &mut self,
        r1: &MoveRecord,
        r2: &MoveRecord,
        start: bool,
        tuning: &StrafeTuning,
        max_speed: f32,
    ) -> Option<f32> {
        let ticks = time_to_ticks(r1.sim_time - r2.sim_time).max(1);
        let mut yaw = normalize_angle(r1.direction.yaw() - r2.direction.yaw());
        if max_speed != 0.0 && r1.mode != Mode::Air {
            yaw *= (r1.velocity.length_2d() / max_speed).clamp(0.0, 1.0);
        }
        if yaw.abs() > 45.0 {
            return None;
        }

        let last_sign = self.sign;
        if yaw != 0.0 {
            self.sign = if yaw > 0.0 { 1 } else { -1 };
        }
        let last_zero = self.zero;
        self.zero = yaw == 0.0;

        let changed = self.sign != last_sign || (self.zero && last_zero);
        let straight = yaw.abs() * r1.velocity.length_2d() * (ticks as f32) < tuning.straight_fuzzy;

        if start {
            self.changes = 0;
            self.start = time_to_ticks(r1.sim_time);
            if straight {
                self.changes += 1;
                if self.changes > tuning.max_changes {
                    return None;
                }
            }
            return Some(yaw);
        }
        if changed || straight {
            self.changes += 1;
            if self.changes > tuning.max_changes {
                return None;
            }
        }
        if self.changes != 0 && self.start - time_to_ticks(r2.sim_time) > tuning.max_change_time {
            return None;
        }
        Some(yaw)
    }
}

/// Average turn per tick over the recent records, zero when they aren't turning steadily
fn average_yaw(records: &VecDeque<MoveRecord>, ground: bool, max_speed: f32, distance: f32) -> f32 {
    // Tuning for how they're moving now, each pair of records uses the one for how they moved
    let current = if ground { &GROUND } else { &AIR };
    let samples = current.samples.min(records.len());
    let mut tracker = YawTracker::default();

    let (mut total, mut ticks, mut skips) = (0.0f32, 0i32, 0usize);
    let mut min_strafes = 4;
    let mut i = 1;
    while i < samples {
        let (r1, r2) = (&records[i - 1], &records[i]);
        if r1.mode != r2.mode {
            skips += 1;
            i += 1;
            continue;
        }
        let pair = if r1.mode != Mode::Air { &GROUND } else { &AIR };
        min_strafes = 4 + pair.max_changes as usize;
        let Some(yaw) = tracker.difference(r1, r2, ticks == 0, pair, max_speed) else {
            break;
        };
        total += yaw;
        ticks += time_to_ticks(r1.sim_time - r2.sim_time).max(1);
        i += 1;
    }
    if i <= min_strafes + skips {
        return 0.0;
    }

    let minimum = if distance < current.low_min_distance {
        current.low_min_samples
    } else {
        let t = (distance - current.low_min_distance)
            / (current.high_min_distance - current.low_min_distance).max(1.0);
        let lo = current.low_min_samples + 1.0;
        (lo + t.clamp(0.0, 1.0) * (current.high_min_samples - lo)).max(1.0)
    };
    let average = total / (ticks as f32).max(minimum);
    if average.abs() < 0.36 { 0.0 } else { average }
}

/// The game's standing and ducked hull mins and maxs
const HULLS: [f32; 12] = [
    -24.0, -24.0, 0.0, 24.0, 24.0, 82.0, -24.0, -24.0, 0.0, 24.0, 24.0, 62.0,
];

/// Where the game keeps `HULLS`, shrunk while simulating to allow for origin rounding
fn view_vectors() -> Option<*mut [f32; 12]> {
    static VECTORS: OnceLock<usize> = OnceLock::new();
    let addr = *VECTORS.get_or_init(|| {
        let bytes: Vec<u8> = HULLS.iter().flat_map(|f| f.to_ne_bytes()).collect();
        let found = module::find_bytes_writable(&module::segments("client.so"), &bytes);
        if found.is_none() {
            log::warn!("Hull view vectors not found, simulated players keep their full size");
        }
        found.unwrap_or(0)
    });
    (addr != 0).then_some(addr as *mut [f32; 12])
}

fn set_hulls(shrink: f32) {
    if let Some(v) = view_vectors() {
        let mut hulls = HULLS;
        for (i, h) in hulls.iter_mut().enumerate() {
            // mins grow inwards, maxs shrink inwards
            *h += if (i / 3) % 2 == 0 { shrink } else { -shrink };
        }
        unsafe { *v = hulls };
    }
}

/// A dummy command for m_pCurrentCommand, game movement only reads a few fields
fn dummy_cmd() -> *mut c_void {
    static CMD: OnceLock<usize> = OnceLock::new();
    *CMD.get_or_init(|| Box::into_raw(Box::new([0u8; 0x80])) as usize) as *mut c_void
}

/// A player being simulated. Their real state comes back when this is dropped. Only one can be
/// alive at a time, they share the game's hull sizes.
pub struct MoveSim {
    player: Player,
    local: bool,
    data: MoveData,
    snapshot: Snapshot,
    average_yaw: f32,
    direct_move: bool,
    bunny_hop: bool,
    sim_time: f32,
    predicted_delta: f32,
    predicted_sim_time: f32,
    predicted_origin: Vec3,
    choke: i32,
    /// Origin before each simulated tick
    pub path: Vec<Vec3>,
}

impl MoveSim {
    /// Start simulating `player`. For the local player pass the command being built, its movement
    /// is used as is, and `bunny_hop` to keep jumping while jump is held.
    pub fn new(player: &Player, local: Option<(&UserCmd, bool)>, strafe: Strafe) -> Option<Self> {
        if player.is_dead() || !Interfaces::move_helper().is_valid() {
            return None;
        }
        let snapshot = Snapshot::take(player.this, player.pred_desc_map())?;
        // Held until the sim is set up, read in place rather than copied out
        let history = HISTORY.lock().unwrap();
        let h = history.get(player.index() as usize)?;
        let choke = if h.update_tick > 0 {
            (Interfaces::global_vars().tickcount - h.update_tick).clamp(0, 24)
        } else {
            0
        };

        let mut sim = MoveSim {
            player: *player,
            local: local.is_some(),
            data: MoveData::default(),
            snapshot,
            average_yaw: 0.0,
            direct_move: true,
            bunny_hop: false,
            sim_time: 0.0,
            predicted_delta: tick_interval(),
            predicted_sim_time: 0.0,
            predicted_origin: Vec3::default(),
            choke,
            path: Vec::new(),
        };
        // Other players' origins are rounded, shrink them so they don't get stuck on walls. Put
        // back when the sim is dropped
        if !sim.local {
            set_hulls(0.125);
        }

        if local.is_none() {
            if let Some(v) = h.avg_velocity {
                player.set_velocity(v);
            }
        }
        // Ducked players have FL_DUCKING without m_bDucked mid transition, which offsets the origin
        let ducking = player.is_ducking();
        player.set_ducked(ducking);
        if ducking {
            player.set_flags(player.flags() & !FL_DUCKING);
            player.reset_duck_transition();
        }
        if let Some((cmd, bhop)) = local {
            sim.bunny_hop = bhop && (cmd.buttons & Buttons::InJump as i32) != 0;
        } else {
            // Leftover base velocity throws them off
            player.set_base_velocity(Vec3::default());
            if player.is_on_ground() {
                let mut v = player.velocity();
                v.z = v.z.min(0.0);
                player.set_velocity(v);
                // Moving faster than their max speed allows, keep up with how they really move
                let speed = v.length_2d();
                if speed > player.max_speed() * 1.02 {
                    player.set_max_speed(speed);
                }
            } else {
                // Otherwise the first tick snaps them back onto the ground
                player.clear_ground_entity();
            }
        }

        sim.setup(local.map(|l| l.0), h.records.front());
        if !h.deltas.is_empty() {
            sim.predicted_delta = h.deltas.iter().sum::<f32>() / h.deltas.len() as f32;
        }
        sim.sim_time = player.simulation_time();
        sim.predicted_sim_time = sim.sim_time + sim.predicted_delta;

        if local.is_none() {
            let enabled = if sim.direct_move {
                strafe.ground
            } else {
                strafe.air
            };
            if enabled {
                let distance = helpers::get_localplayer()
                    .map_or(0.0, |l| l.origin().distance_to(&player.origin()));
                sim.average_yaw =
                    average_yaw(&h.records, sim.direct_move, player.max_speed(), distance);
            }
            drop(history);
            // Catch up on the ticks they've moved since the last update we got
            for _ in 0..choke {
                sim.run_tick();
            }
        }
        Some(sim)
    }

    fn setup(&mut self, cmd: Option<&UserCmd>, record: Option<&MoveRecord>) {
        let p = &self.player;
        let d = &mut self.data;
        d.player_handle = p.ref_handle();
        d.abs_origin = p.network_origin();
        d.velocity = p.velocity();
        d.max_speed = p.max_speed();
        d.client_max_speed = d.max_speed;

        if let Some(cmd) = cmd {
            d.view_angles = cmd.view_angles;
            d.forward_move = cmd.forward_move;
            d.side_move = cmd.side_move;
            d.up_move = cmd.up_move;
            d.buttons = cmd.buttons & !(Buttons::InJump as i32);
        } else if d.velocity.length_2d() > 0.0 {
            d.view_angles = if p.in_cond(Cond::ShieldCharge) {
                Vec3::new(p.eye_pitch(), p.eye_yaw(), 0.0)
            } else {
                Vec3::new(0.0, d.velocity.yaw(), 0.0)
            };
            if let Some(r) = record.filter(|r| !r.direction.is_zero()) {
                let mut m = Movement {
                    forward: r.direction.x,
                    side: -r.direction.y,
                    up: r.direction.z,
                };
                m.fix(&Vec3::default(), &d.view_angles);
                d.forward_move = m.forward;
                d.side_move = m.side;
                d.up_move = m.up;
            }
        }

        d.angles = d.view_angles;
        d.old_angles = d.view_angles;
        let constraint = p.constraint_entity();
        d.constraint_center = Interfaces::entity_list()
            .get_client_entity::<Entity>(handle_index(constraint))
            .filter(|_| constraint != u32::MAX)
            .map_or(p.constraint_center(), |e| e.origin());
        d.constraint_radius = p.constraint_radius();
        d.constraint_width = p.constraint_width();
        d.constraint_speed_factor = p.constraint_speed_factor();

        self.predicted_origin = d.abs_origin;
        self.direct_move = p.is_on_ground() || p.is_swimming();
    }

    /// Advance one tick
    pub fn run_tick(&mut self) {
        self.path.push(self.data.abs_origin);
        let tick = tick_interval();
        let p = self.player;

        let mut correction = 0.0;
        if self.average_yaw != 0.0 {
            let mut mult = 1.0;
            if !self.direct_move && !p.in_cond(Cond::ShieldCharge) {
                correction = 90.0 * self.average_yaw.signum();
                mult = friction_scale(
                    self.data.velocity.length_2d(),
                    self.average_yaw,
                    self.data.velocity.z + gravity() * tick,
                    50.0,
                    150.0,
                );
            }
            self.data.view_angles.y += self.average_yaw * mult + correction;
        } else if !self.direct_move && !self.local {
            self.data.forward_move = 0.0;
            self.data.side_move = 0.0;
        }

        let old_speed = self.data.client_max_speed;
        if p.ducked() && p.is_on_ground() && !p.is_swimming() {
            self.data.client_max_speed /= 3.0;
        }
        if self.bunny_hop && p.is_on_ground() && !p.ducked() {
            self.data.old_buttons = 0;
            self.data.buttons |= Buttons::InJump as i32;
        }

        self.process();
        self.data.client_max_speed = old_speed;

        self.sim_time += tick;
        if self.sim_time >= self.predicted_sim_time {
            self.predicted_origin = self.data.abs_origin;
            self.predicted_sim_time += self.predicted_delta;
        }
        let was_direct = self.direct_move;
        self.direct_move = p.is_on_ground() || p.is_swimming();

        if self.average_yaw != 0.0 {
            self.data.view_angles.y -= correction;
        } else if self.direct_move
            && !was_direct
            && self.data.forward_move == 0.0
            && self.data.side_move == 0.0
            && self.data.velocity.length_2d() > self.data.max_speed * 0.015
        {
            // Landed without input, keep going the way they were flying
            let dir = self.data.velocity.to_2d().normalized() * 450.0;
            let mut m = Movement {
                forward: dir.x,
                side: -dir.y,
                up: 0.0,
            };
            m.fix(&Vec3::default(), &self.data.view_angles);
            self.data.forward_move = m.forward;
            self.data.side_move = m.side;
        }
    }

    /// One ProcessMovement, with prediction set up the way it is when the game predicts
    fn process(&mut self) {
        let prediction = Interfaces::prediction();
        let globals = Interfaces::global_vars_mut();
        let (in_prediction, first_time) = (
            prediction.in_prediction(),
            prediction.first_time_predicted(),
        );
        let frametime = unsafe { (*globals).frametime };

        // Not the first time, so no sounds or effects
        prediction.set_in_prediction(true);
        prediction.set_first_time_predicted(false);
        unsafe {
            (*globals).frametime = if prediction.engine_paused() {
                0.0
            } else {
                Interfaces::global_vars().interval_per_tick
            }
        };
        let helper = Interfaces::move_helper();
        helper.set_host(self.player.this);
        self.player.set_current_command(dummy_cmd());

        Interfaces::game_movement().process_movement(self.player.this, &mut self.data);

        self.player.set_current_command(std::ptr::null_mut());
        helper.set_host(std::ptr::null_mut());
        prediction.set_in_prediction(in_prediction);
        prediction.set_first_time_predicted(first_time);
        unsafe { (*globals).frametime = frametime };
    }

    /// Where the player is after the ticks run so far
    pub fn origin(&self) -> Vec3 {
        self.data.abs_origin
    }

    /// Where the server last got the player to, they only move there when their packets arrive
    pub fn predicted_origin(&self) -> Vec3 {
        self.predicted_origin
    }

    pub fn average_yaw(&self) -> f32 {
        self.average_yaw
    }

    /// Ticks run to catch up to now when the sim started
    pub fn choke(&self) -> i32 {
        self.choke
    }

    /// Average time between the player's updates
    pub fn predicted_delta(&self) -> f32 {
        self.predicted_delta
    }

    pub fn on_ground(&self) -> bool {
        self.player.is_on_ground()
    }
}

impl Drop for MoveSim {
    fn drop(&mut self) {
        if !self.local {
            set_hulls(0.0);
        }
        self.snapshot.restore();
    }
}
