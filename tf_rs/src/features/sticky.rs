//! Auto detonate stickies.
//!
//! Our armed stickies are checked against where enemies will be once the server runs this
//! command. If one would catch an enemy in its blast with nothing in the way, secondary attack
//! detonates them. With the Scottish Resistance only stickies near the crosshair go off, so the
//! view is turned to the one that would hit.

use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::{Lazy, OnceCell};

use crate::{
    config::Config,
    features::{
        movesim::{MoveSim, Strafe},
        projectile,
    },
    helpers,
    interfaces::{Interfaces, mask},
    types::{ClassId, ConVar, Entity, Player, Ray, TraceFilter, UserCmd, Vec3, Weapon},
};

const IN_ATTACK2: i32 = 1 << 11;

const DAMAGE_RADIUS: usize = 0xC14;
const TYPE: usize = 0xC58;
/// TF_GL_MODE_REMOTE_DETONATE and its practice variant
const REMOTE_DETONATE: [i32; 2] = [1, 2];

/// When each sticky, by entity handle, was first seen, the client doesn't know when it was fired
static FIRST_SEEN: Lazy<Mutex<HashMap<u32, f32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

struct Sticky {
    origin: Vec3,
    radius: f32,
    /// Only goes off when looked at, the Scottish Resistance
    needs_aim: bool,
}

fn live_time() -> f32 {
    static LIVE_TIME: OnceCell<Option<ConVar>> = OnceCell::new();
    LIVE_TIME
        .get_or_init(|| Interfaces::cvar().find_var("tf_grenadelauncher_livetime"))
        .as_ref()
        .map_or(0.8, |c| c.float())
}

fn read<T: Copy>(ent: &Entity, offset: usize) -> T {
    unsafe { *ent.field::<T>(offset) }
}

/// Our stickies that can be detonated now
fn armed_stickies(localplayer: &Player, now: f32) -> Vec<Sticky> {
    let list = Interfaces::entity_list();
    let local = localplayer.index();
    let mut first_seen = FIRST_SEEN.lock().unwrap();
    let mut seen = Vec::new();
    let mut out = Vec::new();

    for (handle, ent) in projectile::our_projectiles(local) {
        if !matches!(ent.class_id(), Some(ClassId::PillOrSticky))
            || ent.thrower_index() != local
            || !REMOTE_DETONATE.contains(&read::<i32>(&ent, TYPE))
        {
            continue;
        }
        seen.push(handle);
        let fired = *first_seen.entry(handle).or_insert(now);

        let launcher = list.get_client_entity::<Weapon>(ent.launcher_index());
        let (arm_time, needs_aim) = match &launcher {
            Some(w) => (
                w.attrib_float(live_time(), c"sticky_arm_time"),
                w.attrib_float(0.0, c"set_detonate_mode") as i32 == 1,
            ),
            None => (live_time(), false),
        };
        if now < fired + arm_time {
            continue;
        }
        out.push(Sticky {
            origin: ent.origin(),
            radius: read::<f32>(&ent, DAMAGE_RADIUS),
            needs_aim,
        });
    }
    first_seen.retain(|h, _| seen.contains(h));
    out
}

/// Point of the box `origin + mins..maxs` closest to `point`
fn nearest_point(point: &Vec3, origin: &Vec3, mins: &Vec3, maxs: &Vec3) -> Vec3 {
    Vec3::new(
        point.x.clamp(origin.x + mins.x, origin.x + maxs.x),
        point.y.clamp(origin.y + mins.y, origin.y + maxs.y),
        point.z.clamp(origin.z + mins.z, origin.z + maxs.z),
    )
}

/// Whether a blast at `sticky` reaches a player standing at `origin`, the way the server's
/// radius damage checks it
fn reaches(sticky: &Sticky, origin: &Vec3, mins: &Vec3, maxs: &Vec3) -> bool {
    let point = nearest_point(&sticky.origin, origin, mins, maxs);
    if sticky.origin.distance_to(&point) > sticky.radius * 0.95 {
        return false;
    }
    let filter = TraceFilter::world_only();
    let center = *origin + Vec3::new(0.0, 0.0, (mins.z + maxs.z) / 2.0);
    [point, center].iter().any(|to| {
        let trace =
            Interfaces::engine_trace().trace(&Ray::init(&sticky.origin, to), mask::SOLID, &filter);
        trace.fraction >= 1.0
    })
}

pub fn run(localplayer: &Player, cmd: &mut UserCmd, config: &Config) {
    let aim = &config.aimbot;
    let now = localplayer.curtime();
    let stickies = armed_stickies(localplayer, now);
    if stickies.is_empty() {
        return;
    }
    let reach = stickies.iter().map(|s| s.radius).fold(0.0, f32::max);

    let tick = Interfaces::global_vars().interval_per_tick;
    let latency_ticks = (helpers::round_trip() / tick).round() as i32;
    let strafe = Strafe::from(aim);

    for (_, player) in helpers::enemies(localplayer, aim.ignore_invis) {
        let current = player.origin();
        // Nowhere near any of them, even after moving for a while
        if stickies
            .iter()
            .all(|s| s.origin.distance_to(&current) > reach + 300.0)
        {
            continue;
        }

        // Where they'll be when the server detonates
        let origin = match MoveSim::new(&player, None, strafe) {
            Some(mut sim) => {
                for _ in 0..latency_ticks {
                    sim.run_tick();
                }
                sim.predicted_origin()
            }
            None => current,
        };
        let (mins, maxs) = (player.mins(), player.maxs());

        if let Some(sticky) = stickies.iter().find(|s| reaches(s, &origin, &mins, &maxs)) {
            if sticky.needs_aim {
                cmd.view_angles = helpers::calculate_angle(&localplayer.eye_pos(), &sticky.origin);
            }
            cmd.buttons |= IN_ATTACK2;
            return;
        }
    }
}
