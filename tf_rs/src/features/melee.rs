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
    features::backtrack::{self, Choice},
    globals::{Globals, Target},
    helpers,
    interfaces::Interfaces,
    types::{Player, UserCmd, Vec3, Weapon, user_cmd::Buttons},
};

/// A collision box at a position
#[derive(Clone, Copy)]
pub struct Hull {
    pub origin: Vec3,
    pub mins: Vec3,
    pub maxs: Vec3,
}

impl Hull {
    pub fn of(player: &Player) -> Self {
        Hull {
            origin: player.origin(),
            mins: player.mins(),
            maxs: player.maxs(),
        }
    }

    /// The point of the box to aim at from `eye`, its axis at the eye's height where it can be
    fn aim_point(&self, eye: &Vec3) -> Vec3 {
        let z = (eye.z - self.origin.z).clamp(self.mins.z, self.maxs.z);
        Vec3::new(self.origin.x, self.origin.y, self.origin.z + z)
    }

    fn moved(&self, velocity: &Vec3, time: f32) -> Self {
        Hull {
            origin: add_scaled(&self.origin, velocity, time),
            ..*self
        }
    }
}

/// A melee swing as the server traces it
pub struct Swing {
    start: Vec3,
    end: Vec3,
    hull: f32,
}

impl Swing {
    const HULL: f32 = 18.0;

    /// The swing `weapon` would trace along `angles` if it landed `lead` seconds from now,
    /// with us moving at our current velocity until then
    pub fn new(localplayer: &Player, weapon: &Weapon, angles: &Vec3, lead: f32) -> Option<Self> {
        if !weapon.is_melee() {
            return None;
        }
        let range = weapon.attrib_float(weapon.swing_range(), c"melee_range_multiplier");
        let hull = weapon.attrib_float(Self::HULL, c"melee_bounds_multiplier");
        let start = eye_at(localplayer, lead);
        let end = add_scaled(&start, &forward(angles), range);
        Some(Swing { start, end, hull })
    }

    pub fn hits(&self, target: &Hull) -> bool {
        // Origins are networked with a little less precision than they're stored with
        let shrink = 0.125;
        let mins = Vec3::new(
            target.origin.x + target.mins.x + shrink,
            target.origin.y + target.mins.y + shrink,
            target.origin.z + target.mins.z + shrink,
        );
        let maxs = Vec3::new(
            target.origin.x + target.maxs.x - shrink,
            target.origin.y + target.maxs.y - shrink,
            target.origin.z + target.maxs.z - shrink,
        );
        // Sweeping the cube is the same as a line against the box grown by the cube, and the
        // line trace the server tries first can only hit inside that
        segment_hits_box(&self.start, &self.end, &mins, &maxs, self.hull)
    }
}

/// Where our eye will be `lead` seconds from now, plus the command's own tick of movement that
/// the server applies before the weapon runs
fn eye_at(localplayer: &Player, lead: f32) -> Vec3 {
    let tick = Interfaces::global_vars().interval_per_tick;
    add_scaled(&localplayer.eye_pos(), &localplayer.velocity(), lead + tick)
}

/// CTFKnife::IsBehindAndFacingTarget from `eye` looking along `angles`, at a target standing at
/// `target` and looking along `target_yaw`
fn can_backstab(eye: &Vec3, angles: &Vec3, target: &Vec3, target_yaw: f32) -> bool {
    // Origins are networked to 1/16th of a unit
    const COMPRESSION: f32 = 0.0625;

    let (mut dx, mut dy) = (target.x - eye.x, target.y - eye.y);
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < COMPRESSION * std::f32::consts::SQRT_2 {
        return false;
    }
    (dx, dy) = (dx / dist, dy / dist);
    let extra = 2.0 * COMPRESSION / dist;

    let flat = |v: Vec3| {
        let len = (v.x * v.x + v.y * v.y).sqrt().max(f32::EPSILON);
        (v.x / len, v.y / len)
    };
    let owner = flat(forward(angles));
    let victim = flat(forward(&Vec3::new(0.0, target_yaw, 0.0)));

    let behind = dx * victim.0 + dy * victim.1;
    let facing = dx * owner.0 + dy * owner.1;
    let not_facestab = victim.0 * owner.0 + victim.1 * owner.1;

    behind > 0.0031 + extra && facing > 0.5 + extra && not_facestab > -0.3 + 0.0031
}

/// Whether the swing in progress was started with the aimbot, so it keeps aiming until it lands
static AIMED_SWING: AtomicBool = AtomicBool::new(false);

struct Found {
    index: i32,
    angles: Vec3,
    sim_time: Option<f32>,
    distance: f32,
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
    let found = find_target(localplayer, weapon, cmd, config, lead, backstab_only);
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

/// The closest enemy a swing landing `lead` seconds from now would connect with, aiming at it
fn find_target(
    localplayer: &Player,
    weapon: &Weapon,
    cmd: &UserCmd,
    config: &Config,
    lead: f32,
    backstab_only: bool,
) -> Option<Found> {
    let aim = &config.aimbot;
    let eye = eye_at(localplayer, lead);
    let mut best: Option<Found> = None;

    for i in 1..=Interfaces::engine_client().get_max_clients() {
        let Some(player) = Interfaces::entity_list().get_client_entity::<Player>(i) else {
            continue;
        };
        if player == *localplayer
            || player.is_dormant()
            || player.is_dead()
            || player.team() == localplayer.team()
            || (aim.ignore_invis && player.is_invisible())
        {
            continue;
        }

        // Where the server will check: backtrack records still usable when the swing lands,
        // otherwise where the player is, moved along with how they've been moving
        let mut candidates: Vec<(Hull, Option<f32>)> = backtrack::candidates(i, lead, config)
            .into_iter()
            .map(|(hull, sim_time)| (hull, Some(sim_time)))
            .collect();
        if candidates.is_empty() {
            let velocity = backtrack::velocity(i).unwrap_or_default();
            candidates.push((Hull::of(&player).moved(&velocity, lead), None));
        }

        let target_yaw = player.eye_yaw();
        for (hull, sim_time) in candidates {
            let point = hull.aim_point(&eye);
            let angles = helpers::calculate_angle(&eye, &point);
            if cmd.view_angles.fov_to(&angles) > aim.fov as f32 {
                continue;
            }
            let Some(swing) = Swing::new(localplayer, weapon, &angles, lead) else {
                return None;
            };
            if !swing.hits(&hull) {
                continue;
            }
            if backstab_only && !can_backstab(&eye, &angles, &hull.origin, target_yaw) {
                continue;
            }

            let (dx, dy, dz) = (point.x - eye.x, point.y - eye.y, point.z - eye.z);
            let distance = dx * dx + dy * dy + dz * dz;
            // Same distance, the newer record relies less on timing
            let better = best.as_ref().is_none_or(|b| {
                distance < b.distance
                    || (distance == b.distance && sim_time > b.sim_time && b.index == i)
            });
            if better {
                best = Some(Found {
                    index: i,
                    angles,
                    sim_time,
                    distance,
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

/// Whether the segment from `start` to `end` passes through the box grown by `grow` on each side
fn segment_hits_box(start: &Vec3, end: &Vec3, mins: &Vec3, maxs: &Vec3, grow: f32) -> bool {
    let axes = [
        (start.x, end.x, mins.x, maxs.x),
        (start.y, end.y, mins.y, maxs.y),
        (start.z, end.z, mins.z, maxs.z),
    ];
    let (mut enter, mut exit) = (0.0f32, 1.0f32);
    for (s, e, lo, hi) in axes {
        let (lo, hi) = (lo - grow, hi + grow);
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
