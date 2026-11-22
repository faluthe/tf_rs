//! Melee aimbot, swing prediction and auto backstab.
//!
//! The server decides a melee hit when the swing lands (the smack, ~0.2s after the swing starts,
//! instantly for knives) by tracing from the eye along the view angles for the weapon's range,
//! first as a line then as an 18 unit cube, against players' collision boxes. The aimbot looks
//! for a box that trace connects with, aims at it on every command until the smack, and with
//! swing prediction starts swinging when the target will be in reach by the time it lands.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    config::Config,
    features::{
        backtrack::{self, Choice},
        engine_prediction,
        movesim::{MoveSim, Strafe},
    },
    globals::{Globals, Target},
    helpers,
    interfaces::Interfaces,
    types::{Hull, Player, UserCmd, Vec3, Weapon, user_cmd::Buttons},
};

impl Hull {
    /// The point of the box to aim at from `eye`, its axis at the eye's height where it can be
    fn aim_point(&self, eye: &Vec3) -> Vec3 {
        let z = (eye.z - self.origin.z).clamp(self.mins.z, self.maxs.z);
        Vec3::new(self.origin.x, self.origin.y, self.origin.z + z)
    }
}

/// A melee swing as the server traces it
#[derive(Clone, Copy)]
pub struct Swing {
    start: Vec3,
    end: Vec3,
    range: f32,
    hull: f32,
}

impl Swing {
    const HULL: f32 = 18.0;

    /// The swing a melee `weapon` would trace from `start` along `angles`
    pub fn new(weapon: &Weapon, start: Vec3, angles: &Vec3) -> Self {
        let range = weapon.attrib_float(weapon.swing_range(), c"melee_range_multiplier");
        let hull = weapon.attrib_float(Self::HULL, c"melee_bounds_multiplier");
        Swing {
            start,
            end: add_scaled(&start, &forward(angles), range),
            range,
            hull,
        }
    }

    /// The same swing traced along `angles` instead
    pub fn along(&self, angles: &Vec3) -> Self {
        Swing {
            end: add_scaled(&self.start, &forward(angles), self.range),
            ..*self
        }
    }

    pub fn hits(&self, target: &Hull) -> bool {
        let (mins, maxs) = self.reach_box(target);
        segment_hits_box(&self.start, &self.end, &mins, &maxs)
    }

    /// Whether no swing from where this one starts can hit `target`, whatever its angles
    pub fn out_of_reach(&self, target: &Hull) -> bool {
        let (mins, maxs) = self.reach_box(target);
        let gap = |s: f32, lo: f32, hi: f32| s - s.clamp(lo, hi);
        let (dx, dy, dz) = (
            gap(self.start.x, mins.x, maxs.x),
            gap(self.start.y, mins.y, maxs.y),
            gap(self.start.z, mins.z, maxs.z),
        );
        dx * dx + dy * dy + dz * dz > self.range * self.range
    }

    /// The box the swing's line has to pass through to hit `target`. Sweeping the cube is the
    /// same as a line against the box grown by the cube, and the line trace the server tries
    /// first can only hit inside that
    fn reach_box(&self, target: &Hull) -> (Vec3, Vec3) {
        // Origins are networked with a little less precision than they're stored with
        let grow = self.hull - 0.125;
        let (o, lo, hi) = (&target.origin, &target.mins, &target.maxs);
        (
            Vec3::new(o.x + lo.x - grow, o.y + lo.y - grow, o.z + lo.z - grow),
            Vec3::new(o.x + hi.x + grow, o.y + hi.y + grow, o.z + hi.z + grow),
        )
    }
}

fn ticks(time: f32) -> i32 {
    (time / Interfaces::global_vars().interval_per_tick).round() as i32
}

/// Where our eye will be `lead` seconds after this command, which the server moves us by before
/// the weapon runs, keeping the command's movement
pub fn eye_at(localplayer: &Player, cmd: &UserCmd, lead: f32, config: &Config) -> Vec3 {
    // The command's own tick, unless engine prediction has already run it
    let own = if engine_prediction::active() { 0 } else { 1 };
    if let Some(mut sim) = MoveSim::new(localplayer, Some((cmd, config.bunnyhop)), Strafe::NONE) {
        for _ in 0..own + ticks(lead) {
            sim.run_tick();
        }
        // Read before the sim restores it, ducking moves the view
        return sim.origin() + localplayer.view_offset();
    }
    let tick = Interfaces::global_vars().interval_per_tick;
    add_scaled(
        &localplayer.eye_pos(),
        &localplayer.velocity(),
        lead + own as f32 * tick,
    )
}

/// Where the server will have `player` when a swing lands `lead` seconds from now
fn predicted_hull(player: &Player, lead: f32, strafe: Strafe) -> Option<Hull> {
    let mut sim = MoveSim::new(player, None, strafe)?;
    for _ in 0..ticks(lead) {
        sim.run_tick();
    }
    Some(Hull {
        origin: sim.predicted_origin(),
        mins: player.mins(),
        maxs: player.maxs(),
    })
}

/// CTFKnife::IsBehindAndFacingTarget from `eye` looking along `angles`, at a target standing at
/// `target` and looking along `target_yaw`
pub fn can_backstab(eye: &Vec3, angles: &Vec3, target: &Vec3, target_yaw: f32) -> bool {
    Backstab::new(eye, target, target_yaw).is_some_and(|b| b.slack(angles).is_some())
}

/// The part of the knife's backstab check that doesn't depend on our angles
struct Backstab {
    /// Flat direction from the eye to the target
    to_target: (f32, f32),
    /// Flat direction the target is looking
    victim: (f32, f32),
    /// Extra margin for the origins' networked precision
    extra: f32,
}

impl Backstab {
    /// None when the target isn't facing away from `eye`, no angles can backstab it then
    fn new(eye: &Vec3, target: &Vec3, target_yaw: f32) -> Option<Self> {
        // Origins are networked to 1/16th of a unit
        const COMPRESSION: f32 = 0.0625;

        let (dx, dy) = (target.x - eye.x, target.y - eye.y);
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < COMPRESSION * std::f32::consts::SQRT_2 {
            return None;
        }
        let to_target = (dx / dist, dy / dist);
        let extra = 2.0 * COMPRESSION / dist;
        let victim = flat(forward(&Vec3::new(0.0, target_yaw, 0.0)));

        let behind = to_target.0 * victim.0 + to_target.1 * victim.1 - (0.0031 + extra);
        (behind > 0.0).then_some(Backstab {
            to_target,
            victim,
            extra,
        })
    }

    /// How far from failing the checks that depend on `angles` are, none when one fails
    fn slack(&self, angles: &Vec3) -> Option<f32> {
        let owner = flat(forward(angles));
        let facing = self.to_target.0 * owner.0 + self.to_target.1 * owner.1 - (0.5 + self.extra);
        let not_facestab = self.victim.0 * owner.0 + self.victim.1 * owner.1 - (-0.3 + 0.0031);
        (facing > 0.0 && not_facestab > 0.0).then_some(facing.min(not_facestab))
    }
}

/// Direction of `v` on the ground plane
fn flat(v: Vec3) -> (f32, f32) {
    let len = (v.x * v.x + v.y * v.y).sqrt().max(f32::EPSILON);
    (v.x / len, v.y / len)
}

/// Angles to backstab `hull` with `swing`, turned from aiming at its axis (`angles`) to
/// wherever the swing still connects with the most room on both of the knife's angle checks
fn backstab_angles(swing: &Swing, angles: &Vec3, hull: &Hull, target_yaw: f32) -> Option<Vec3> {
    // Facing the target needs to be within 60 degrees of it
    const SPREAD: i32 = 60;
    const OFFSETS: usize = 2 * SPREAD as usize + 3;

    let backstab = Backstab::new(&swing.start, &hull.origin, target_yaw)?;
    let at = |i: usize| {
        Vec3::new(
            angles.x,
            angles.y + (i as i32 - SPREAD - 1) as f32,
            angles.z,
        )
    };
    let mut slack = [None; OFFSETS];
    for (i, slack) in slack.iter_mut().enumerate() {
        let angles = at(i);
        if swing.along(&angles).hits(hull) {
            *slack = backstab.slack(&angles);
        }
    }

    // Stay a degree inside the edges so a little error in either position doesn't miss it
    (1..OFFSETS - 1)
        .filter(|&i| slack[i - 1].is_some() && slack[i + 1].is_some())
        .filter_map(|i| Some((i, slack[i]?)))
        .max_by(|a, b| {
            let off = |i: usize| (i as i32 - SPREAD - 1).abs();
            a.1.total_cmp(&b.1).then(off(b.0).cmp(&off(a.0)))
        })
        .map(|(i, _)| at(i))
}

/// Whether the swing in progress was started with the aimbot, so it keeps aiming until it lands
static AIMED_SWING: AtomicBool = AtomicBool::new(false);

struct Found {
    index: i32,
    angles: Vec3,
    sim_time: Option<f32>,
    distance: f32,
    backstab: bool,
}

/// Aim and swing with a melee weapon, returns which tick backtrack should use
pub fn run(localplayer: &Player, weapon: &Weapon, cmd: &mut UserCmd, config: &Config) -> Choice {
    let aim = &config.aimbot;
    let knife = weapon.is_knife();
    let now = localplayer.curtime();

    // Mid swing the hit lands at the smack, otherwise a swing started now lands after the delay
    let smack_time = weapon.smack_time();
    let swinging = smack_time > 0.0;
    if !swinging {
        AIMED_SWING.store(false, Ordering::Relaxed);
    }
    let can_attack = localplayer.can_attack();
    if !swinging && !can_attack {
        Globals::write().target = None;
        return Choice::Auto;
    }
    let lead = if swinging {
        (smack_time - now).max(0.0)
    } else if knife || !aim.swing_prediction {
        0.0
    } else {
        weapon.smack_delay()
    };

    let backstab_only = knife && aim.auto_backstab;
    let found = find_target(localplayer, weapon, cmd, config, lead, knife, backstab_only);
    Globals::write().target = found.as_ref().map(|f| Target {
        target_index: f.index,
        should_headshot: false,
    });
    let Some(found) = found else {
        return Choice::Auto;
    };

    let key_down = Globals::read().aimbot_key_down;
    let wants = if aim.key.use_key {
        key_down
    } else {
        (cmd.buttons & Buttons::InAttack as i32) != 0
    };
    let auto_swing = aim.auto_swing && (!aim.key.use_key || key_down);
    let attack = !swinging && can_attack && (auto_swing || backstab_only);

    // Keep aiming through a swing we started, the angles that count are the ones at the smack
    let aimed_swing = swinging && AIMED_SWING.load(Ordering::Relaxed);
    if !(wants || attack || aimed_swing) {
        return Choice::Auto;
    }
    cmd.view_angles = found.angles;
    if attack {
        cmd.buttons |= Buttons::InAttack as i32;
    }
    // A swing starting on this command is one we're aiming
    if !swinging && can_attack && (cmd.buttons & Buttons::InAttack as i32) != 0 {
        AIMED_SWING.store(true, Ordering::Relaxed);
    }

    match found.sim_time {
        Some(sim_time) => Choice::Record(found.index, sim_time),
        None => Choice::Keep,
    }
}

/// The closest enemy a swing landing `lead` seconds from now would connect with, aiming at it.
/// With a knife any record that can be backstabbed comes first, aiming the way that stabs it
fn find_target(
    localplayer: &Player,
    weapon: &Weapon,
    cmd: &UserCmd,
    config: &Config,
    lead: f32,
    knife: bool,
    backstab_only: bool,
) -> Option<Found> {
    let aim = &config.aimbot;
    // Only the angles change between swings, the rest is worked out once
    let eye = eye_at(localplayer, cmd, lead, config);
    let swing = Swing::new(weapon, eye, &cmd.view_angles);
    let strafe = Strafe::from(aim);
    // Locked once for every player rather than for each of them
    let lookup = backtrack::Lookup::new(config);
    let mut best: Option<Found> = None;

    for (i, player) in helpers::enemies(localplayer, aim.ignore_invis) {
        // Out of reach even running at each other
        if player.origin().distance_to(&eye) > 600.0 {
            continue;
        }

        // Where the server will check: backtrack records still usable when the swing lands,
        // otherwise where they'll have moved to by then
        let mut records = lookup
            .candidates(i, lead)
            .map(|(hull, sim_time)| (hull, Some(sim_time)))
            .peekable();
        let now = records.peek().is_none().then(|| {
            let hull = predicted_hull(&player, lead, strafe).unwrap_or_else(|| player.hull());
            (hull, None)
        });

        let target_yaw = player.eye_yaw();
        for (hull, sim_time) in records.chain(now) {
            if swing.out_of_reach(&hull) {
                continue;
            }
            let point = hull.aim_point(&eye);
            let angles = helpers::calculate_angle(&eye, &point);
            if cmd.view_angles.fov_to(&angles) > aim.fov as f32 {
                continue;
            }
            let stab = knife
                .then(|| backstab_angles(&swing, &angles, &hull, target_yaw))
                .flatten();
            let (angles, backstab) = match stab {
                Some(stab) => (stab, true),
                None if backstab_only => continue,
                None => {
                    if !swing.along(&angles).hits(&hull) {
                        continue;
                    }
                    (angles, false)
                }
            };

            let (dx, dy, dz) = (point.x - eye.x, point.y - eye.y, point.z - eye.z);
            let distance = dx * dx + dy * dy + dz * dz;
            // Same distance, the newer record relies less on timing
            let better = best.as_ref().is_none_or(|b| {
                (backstab, -distance) > (b.backstab, -b.distance)
                    || (backstab == b.backstab
                        && distance == b.distance
                        && sim_time > b.sim_time
                        && b.index == i)
            });
            if better {
                best = Some(Found {
                    index: i,
                    angles,
                    sim_time,
                    distance,
                    backstab,
                });
            }
        }
    }
    best
}

pub fn forward(angles: &Vec3) -> Vec3 {
    let (sp, cp) = angles.x.to_radians().sin_cos();
    let (sy, cy) = angles.y.to_radians().sin_cos();
    Vec3::new(cp * cy, cp * sy, -sp)
}

fn add_scaled(a: &Vec3, b: &Vec3, scale: f32) -> Vec3 {
    Vec3::new(a.x + b.x * scale, a.y + b.y * scale, a.z + b.z * scale)
}

/// Whether the segment from `start` to `end` passes through the box from `mins` to `maxs`
fn segment_hits_box(start: &Vec3, end: &Vec3, mins: &Vec3, maxs: &Vec3) -> bool {
    let axes = [
        (start.x, end.x, mins.x, maxs.x),
        (start.y, end.y, mins.y, maxs.y),
        (start.z, end.z, mins.z, maxs.z),
    ];
    let (mut enter, mut exit) = (0.0f32, 1.0f32);
    for (s, e, lo, hi) in axes {
        let d = e - s;
        if d.abs() < f32::EPSILON {
            if s < lo || s > hi {
                return false;
            }
            continue;
        }
        let (t0, t1) = ((lo - s) / d, (hi - s) / d);
        enter = enter.max(t0.min(t1));
        exit = exit.min(t0.max(t1));
        if enter > exit {
            return false;
        }
    }
    true
}
