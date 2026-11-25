//! Projectile aimbot.
//!
//! Targets are moved forward tick by tick with movement simulation. At each tick the angle that
//! lands a projectile on them is solved for, and the first tick the projectile gets there in time
//! is the one aimed at. The solve accounts for the projectile starting off to the side of the eye
//! and, for rockets and arrows, being angled in towards the crosshair.
//!
//! Explosives that can't reach a target directly look for a surface near it to hit instead,
//! close enough for the blast to reach them with nothing in between.

use std::{
    cmp::Ordering,
    sync::{Arc, Mutex},
};

use crate::{
    config::Config,
    features::{
        movesim::{self, MoveSim, Strafe},
        shot_log,
    },
    globals::{Globals, PredictionPaths, Target},
    helpers,
    interfaces::{Interfaces, mask},
    types::weapon::WeaponClass,
    types::{
        ClassId, Entity, Flow, Player, Ray, TraceFilter, UserCmd, Vec3, Weapon, user_cmd::Buttons,
    },
};

const MAX_TARGETS: usize = 3;
/// Surface directions looked along around a target for splash
const SPLASH_DIRECTIONS: usize = 32;
/// Closest splash spots tried per tick
const SPLASH_CANDIDATES: usize = 6;
/// Ticks per target splash is looked for, from the first one the projectile could make it by
const SPLASH_TICKS: i32 = 12;

/// Seconds a shot waits for its projectile to show up before it's taken as never fired
const SPAWN_WINDOW: f32 = 0.5;

/// An explosive we fired that's still on its way to a target
struct InFlight {
    target: i32,
    /// Server time it lands, on the same timeline targets are simulated on
    land_time: f32,
    /// Realtime it was fired at
    fired: f32,
    /// Its entity handle once it's shown up
    projectile: Option<u32>,
    /// Realtime by which it should have landed, still flying after that it missed
    expires: f32,
}

#[derive(Default)]
struct Tracking {
    in_flight: Vec<InFlight>,
    /// Our projectiles last time, so new ones can be told apart
    known: Vec<u32>,
}

/// Our explosives in the air. A blast throws its target, so a shot that lands after one of these
/// is aimed at where the target would have been without it.
static TRACKING: Mutex<Tracking> = Mutex::new(Tracking {
    in_flight: Vec::new(),
    known: Vec::new(),
});

/// One of our projectiles, found by a scan
struct Scanned {
    handle: u32,
    index: i32,
}

/// The last scan for our projectiles and the local player it was for. They only come and go with
/// network updates, so one scan serves every caller until `forget_projectiles` at the next one.
static SCANNED: Mutex<Option<(i32, Vec<Scanned>)>> = Mutex::new(None);

/// Projectiles may have come or gone, the next caller scans again
pub fn forget_projectiles() {
    *SCANNED.lock().unwrap() = None;
}

/// Our projectiles that are out, by entity handle
pub fn our_projectiles(local: i32) -> Vec<(u32, Entity)> {
    let list = Interfaces::entity_list();
    let mut scanned = SCANNED.lock().unwrap();
    if scanned.as_ref().is_none_or(|(l, _)| *l != local) {
        *scanned = Some((local, scan_projectiles(local)));
    }
    let (_, found) = scanned.as_ref().unwrap();
    // Looked up again by index, a handle that no longer matches is a different entity now
    found
        .iter()
        .filter_map(|s| {
            let ent = list.get_client_entity::<Entity>(s.index)?;
            (ent.ref_handle() == s.handle).then_some((s.handle, ent))
        })
        .collect()
}

fn scan_projectiles(local: i32) -> Vec<Scanned> {
    let list = Interfaces::entity_list();
    let mut out = Vec::new();
    for i in 1..=list.max_entities() {
        let Some(ent) = list.get_client_entity::<Entity>(i) else {
            continue;
        };
        let class = ent.class_id();
        let projectile = matches!(
            class,
            Some(
                ClassId::Rocket
                    | ClassId::PillOrSticky
                    | ClassId::Arrow
                    | ClassId::Flare
                    | ClassId::CrossbowBolt
            )
        );
        if !projectile || ent.is_dormant() {
            continue;
        }
        let thrower = matches!(class, Some(ClassId::PillOrSticky)) && ent.thrower_index() == local;
        if ent.owner_index() == local || thrower {
            out.push(Scanned {
                handle: ent.ref_handle(),
                index: i,
            });
        }
    }
    out
}

/// Match new projectiles to the shots that made them and forget shots whose projectile is gone
fn track_in_flight(localplayer: &Player) {
    let now = Interfaces::global_vars().realtime;
    let ours: Vec<u32> = our_projectiles(localplayer.index())
        .into_iter()
        .map(|(h, _)| h)
        .collect();
    let mut tracking = TRACKING.lock().unwrap();
    let Tracking { in_flight, known } = &mut *tracking;
    for handle in ours.iter().filter(|h| !known.contains(h)) {
        if let Some(shot) = in_flight.iter_mut().find(|f| f.projectile.is_none()) {
            shot.projectile = Some(*handle);
        }
    }
    in_flight.retain(|f| {
        now < f.expires
            && match f.projectile {
                Some(h) => ours.contains(&h),
                None => now - f.fired < SPAWN_WINDOW,
            }
    });
    *known = ours;
}

/// Target and landing time of each of our explosives in the air. Only ones seen flying count,
/// several commands in a frame can each look like they fire before the weapon's next attack
/// time catches up.
fn landings() -> Vec<(i32, f32)> {
    TRACKING
        .lock()
        .unwrap()
        .in_flight
        .iter()
        .filter(|f| f.projectile.is_some())
        .map(|f| (f.target, f.land_time))
        .collect()
}

/// How a weapon's projectile leaves it
#[derive(Clone, Copy)]
struct ProjectileInfo {
    /// From the eye, forward/right/up
    offset: Vec3,
    /// Fired straight along the view instead of angled in to meet the crosshair
    pipes: bool,
    speed: f32,
    /// Extra upwards launch speed, pipes and stickies get 200
    up_speed: f32,
    /// Units per second squared
    gravity: f32,
    /// Air drag on physics projectiles, the fraction of their speed lost each second per unit
    /// per second they're going. Amalgam's measurements, the engine takes off |v . basis| of the
    /// velocity every second and a spinning pipe averages out to about one basis.
    drag: f32,
    /// Half size of the projectile's box
    hull: f32,
    /// Seconds before it's gone, the simulation stops there
    lifetime: f32,
    /// Explodes, so aim at the feet of players on the ground
    splash: bool,
    splash_radius: f32,
    headshot: bool,
    charge: bool,
}

fn charge_time(weapon: &Weapon, now: f32) -> f32 {
    let begin = weapon.charge_begin_time();
    if begin > 0.0 {
        (now - begin).max(0.0)
    } else {
        0.0
    }
}

fn remap(value: f32, from: (f32, f32), to: (f32, f32)) -> f32 {
    let t = ((value - from.0) / (from.1 - from.0)).clamp(0.0, 1.0);
    to.0 + t * (to.1 - to.0)
}

/// Amalgam's per weapon projectile info, without the vphysics drag on pipes
fn weapon_info(localplayer: &Player, weapon: &Weapon) -> Option<ProjectileInfo> {
    let gravity = movesim::gravity();
    let ducking = localplayer.is_ducking();
    let now = localplayer.curtime();
    let attrib = |value: f32, name: &std::ffi::CStr| weapon.attrib_float(value, name);
    let mut info = ProjectileInfo {
        offset: Vec3::new(23.5, 8.0, if ducking { 8.0 } else { -3.0 }),
        pipes: false,
        speed: 0.0,
        up_speed: 0.0,
        gravity: 0.0,
        drag: 0.0,
        hull: 0.0,
        lifetime: f32::MAX,
        splash: false,
        splash_radius: 0.0,
        headshot: false,
        charge: false,
    };

    match weapon.weapon_class() {
        WeaponClass::Rocketlauncher | WeaponClass::RocketlauncherDirecthit => {
            let center = attrib(0.0, c"centerfire_projectile") as i32 == 1;
            info.offset.y = if center { 0.0 } else { 12.0 };
            info.speed = attrib(1100.0, c"mult_projectile_speed");
            info.splash = true;
            info.splash_radius = attrib(146.0, c"mult_explosion_radius");
        }
        WeaponClass::ParticleCannon | WeaponClass::Raygun | WeaponClass::DrgPomson => {
            let mangler = matches!(weapon.weapon_class(), WeaponClass::ParticleCannon);
            if matches!(weapon.weapon_class(), WeaponClass::DrgPomson) {
                info.offset.z -= 13.0;
            }
            info.speed = if mangler { 1100.0 } else { 1200.0 };
            info.hull = if mangler { 0.0 } else { 1.0 };
            info.splash = mangler;
            info.splash_radius = if mangler { 146.0 } else { 0.0 };
        }
        WeaponClass::Grenadelauncher | WeaponClass::Cannon => {
            info.offset = Vec3::new(16.0, 8.0, -6.0);
            info.pipes = true;
            info.speed = attrib(
                attrib(1200.0, c"mult_projectile_speed"),
                c"mult_projectile_range",
            );
            info.up_speed = 200.0;
            info.gravity = 800.0;
            info.drag = if matches!(weapon.weapon_class(), WeaponClass::Cannon) {
                0.385 / 1454.0
            } else if attrib(0.0, c"grenade_no_spin") != 0.0 {
                // Loch-n-Load, a pipe that doesn't spin
                0.07 / 1504.0
            } else {
                0.12 / 1217.0
            };
            info.hull = 6.0;
            let fuse = attrib(2.0, c"fuse_mult");
            info.lifetime = ((fuse / 0.195).floor() + 1.0) * 0.195;
        }
        WeaponClass::Pipebomblauncher => {
            info.offset = Vec3::new(16.0, 8.0, -6.0);
            info.pipes = true;
            let rate = attrib(4.0, c"stickybomb_charge_rate");
            let speed = remap(charge_time(weapon, now), (0.0, rate), (900.0, 2400.0));
            info.speed = attrib(speed, c"mult_projectile_range");
            info.up_speed = 200.0;
            info.gravity = 800.0;
            info.drag = 0.085 / 922.0;
            info.hull = 6.0;
            info.splash = true;
            info.splash_radius = attrib(146.0, c"mult_explosion_radius");
            info.charge = true;
        }
        WeaponClass::Flaregun => {
            info.offset.y = 12.0;
            info.speed = attrib(2000.0, c"mult_projectile_speed");
            info.gravity = 0.3 * gravity;
        }
        WeaponClass::FlaregunRevenge => {
            info.offset.y = 12.0;
            info.speed = 3000.0;
            info.gravity = 0.45 * gravity;
        }
        WeaponClass::CompoundBow => {
            info.offset = Vec3::new(23.5, 8.0, -3.0);
            let charge = charge_time(weapon, now);
            info.speed = remap(charge, (0.0, 1.0), (1800.0, 2600.0));
            info.gravity = remap(charge, (0.0, 1.0), (0.5, 0.1)) * gravity;
            info.hull = 1.0;
            info.headshot = true;
            info.charge = true;
        }
        WeaponClass::Crossbow | WeaponClass::ShotgunBuildingRescue => {
            info.offset = Vec3::new(23.5, 8.0, -3.0);
            info.speed = 2400.0;
            info.gravity = 0.2 * gravity;
            info.hull = if matches!(weapon.weapon_class(), WeaponClass::Crossbow) {
                3.0
            } else {
                1.0
            };
        }
        _ => return None,
    }

    if flip_viewmodels() {
        info.offset.y = -info.offset.y;
    }
    Some(info)
}

pub fn is_projectile_weapon(weapon: &Weapon) -> bool {
    matches!(
        weapon.weapon_class(),
        WeaponClass::Rocketlauncher
            | WeaponClass::RocketlauncherDirecthit
            | WeaponClass::ParticleCannon
            | WeaponClass::Raygun
            | WeaponClass::DrgPomson
            | WeaponClass::Grenadelauncher
            | WeaponClass::Cannon
            | WeaponClass::Pipebomblauncher
            | WeaponClass::Flaregun
            | WeaponClass::FlaregunRevenge
            | WeaponClass::CompoundBow
            | WeaponClass::Crossbow
            | WeaponClass::ShotgunBuildingRescue
    )
}

fn flip_viewmodels() -> bool {
    use once_cell::sync::OnceCell;
    static FLIP: OnceCell<Option<crate::types::ConVar>> = OnceCell::new();
    FLIP.get_or_init(|| Interfaces::cvar().find_var("cl_flipviewmodels"))
        .is_some_and(|c| c.float() != 0.0)
}

/// Where the projectile starts and which way it heads when fired along `angles`
fn fire_setup(
    localplayer: &Player,
    eye: &Vec3,
    angles: &Vec3,
    info: &ProjectileInfo,
) -> (Vec3, Vec3) {
    let (forward, right, up) = angles.angle_vectors();
    let pos = *eye + forward * info.offset.x + right * info.offset.y + up * info.offset.z;
    if info.pipes {
        return (pos, *angles);
    }
    // Rockets and arrows are angled to meet whatever the crosshair is on
    let mut end = *eye + forward * 2000.0;
    let trace = Interfaces::engine_trace().trace_ray(eye, &end, mask::SOLID, Some(localplayer));
    if trace.did_hit() && trace.fraction > 0.1 {
        end = trace.end_pos;
    }
    (pos, (end - pos).to_angles())
}

/// Launch velocity for a projectile heading along `angles`
fn launch_velocity(angles: &Vec3, info: &ProjectileInfo) -> Vec3 {
    let (forward, _, up) = angles.angle_vectors();
    let v = forward * info.speed + up * info.up_speed;
    // Physics objects can't go faster than the physics speed limit
    if info.pipes && v.length() > MAX_PHYSICS_SPEED {
        return v.normalized() * MAX_PHYSICS_SPEED;
    }
    v
}

const MAX_PHYSICS_SPEED: f32 = 2000.0;

/// One tick of a physics projectile's flight: gravity and drag change its velocity, then it moves
fn physics_step(pos: &mut Vec3, vel: &mut Vec3, dt: f32, info: &ProjectileInfo) {
    vel.z -= info.gravity * dt;
    let slow = (info.drag * vel.length() * dt).min(1.0);
    *vel = *vel * (1.0 - slow);
    *pos += *vel * dt;
}

/// Flights of a physics projectile launched at each whole degree of pitch, as horizontal distance
/// and height every tick, to solve against since drag leaves no formula for the arc
struct ArcTable {
    /// Pitch in degrees, up positive, lowest first
    pitches: Vec<f32>,
    arcs: Vec<Vec<(f32, f32)>>,
}

impl ArcTable {
    fn new(info: &ProjectileInfo, speed: f32, max_time: f32) -> Self {
        let tick = Interfaces::global_vars().interval_per_tick;
        let ticks = (max_time / tick).ceil() as usize;
        let pitches: Vec<f32> = (-89..=89).map(|p| p as f32).collect();
        let arcs = pitches
            .iter()
            .map(|pitch| {
                let (sin, cos) = pitch.to_radians().sin_cos();
                let mut pos = Vec3::default();
                let mut vel = Vec3::new(cos * speed, 0.0, sin * speed);
                let mut arc = Vec::with_capacity(ticks + 1);
                arc.push((0.0, 0.0));
                for _ in 0..ticks {
                    physics_step(&mut pos, &mut vel, tick, info);
                    arc.push((pos.x, pos.z));
                }
                arc
            })
            .collect();
        ArcTable { pitches, arcs }
    }

    /// The table for these flights, reusing the last one built when nothing changed. Building one
    /// is a few hundred ticks for each of 179 pitches, too slow to do every command.
    fn cached(info: &ProjectileInfo, speed: f32, max_time: f32) -> Arc<Self> {
        static LAST: Mutex<Option<([u32; 4], Arc<ArcTable>)>> = Mutex::new(None);
        let tick = Interfaces::global_vars().interval_per_tick;
        let key = [speed, info.drag, info.gravity, (max_time / tick).ceil()].map(f32::to_bits);
        let mut last = LAST.lock().unwrap();
        if let Some((k, table)) = last.as_ref()
            && *k == key
        {
            return table.clone();
        }
        let table = Arc::new(ArcTable::new(info, speed, max_time));
        *last = Some((key, table.clone()));
        table
    }

    /// Height and time an arc is at when it's `distance` out, if it gets that far
    fn at(arc: &[(f32, f32)], distance: f32, tick: f32) -> Option<(f32, f32)> {
        if arc.last()?.0 < distance {
            return None;
        }
        let i = arc.partition_point(|(d, _)| *d < distance);
        if i == 0 {
            return Some((arc[0].1, 0.0));
        }
        let ((d0, z0), (d1, z1)) = (arc[i - 1], arc[i]);
        let a = if d1 > d0 {
            (distance - d0) / (d1 - d0)
        } else {
            0.0
        };
        Some((z0 + (z1 - z0) * a, (i as f32 - 1.0 + a) * tick))
    }

    /// The low arc's pitch (radians, up positive) and flight time to `distance` out and `height` up
    fn solve(&self, distance: f32, height: f32) -> Option<(f32, f32)> {
        let tick = Interfaces::global_vars().interval_per_tick;
        // Height at the distance rises with pitch up to the furthest reaching arc, the first
        // pitch that gets high enough is the low arc
        let mut below: Option<(f32, f32, f32)> = None;
        for (pitch, arc) in self.pitches.iter().zip(&self.arcs) {
            let Some((z, t)) = Self::at(arc, distance, tick) else {
                below = None;
                continue;
            };
            let over = z - height;
            if over >= 0.0 {
                let (pitch, t) = match below {
                    Some((p0, o0, t0)) => {
                        let a = -o0 / (over - o0);
                        (p0 + (pitch - p0) * a, t0 + (t - t0) * a)
                    }
                    None => (*pitch, t),
                };
                return Some((pitch.to_radians(), t));
            }
            below = Some((*pitch, over, t));
        }
        None
    }
}

struct Solver<'a> {
    localplayer: &'a Player,
    eye: Vec3,
    info: ProjectileInfo,
    /// Launch speed including the upwards part
    speed: f32,
    /// Angle the launch velocity makes with the view
    ang_fix: Vec3,
    offset_time: f32,
    /// For projectiles with drag
    arcs: Option<Arc<ArcTable>>,
}

#[derive(Clone, Copy)]
struct Solution {
    angles: Vec3,
    time: f32,
}

enum Solve {
    Good(Solution),
    /// The projectile gets there after the target does
    Late,
    /// Out of reach
    Bad,
}

impl Solver<'_> {
    /// A solver for flights up to `max_time` long
    fn new<'a>(
        localplayer: &'a Player,
        eye: Vec3,
        info: ProjectileInfo,
        max_time: f32,
    ) -> Solver<'a> {
        let v = launch_velocity(&Vec3::default(), &info);
        let (pos, _) = fire_setup(localplayer, &eye, &Vec3::default(), &info);
        let speed = v.length();
        // Splash can land the blast radius short of a target, so flights that much longer count
        let max_time = (max_time + info.splash_radius / speed).min(info.lifetime);
        let arcs = (info.drag > 0.0).then(|| ArcTable::cached(&info, speed, max_time));
        Solver {
            localplayer,
            eye,
            info,
            speed,
            ang_fix: v.to_angles(),
            offset_time: (pos - eye).length() / speed,
            arcs,
        }
    }

    /// Launch pitch (radians, up positive) and flight time from `from` to `to`
    fn trajectory(&self, from: &Vec3, to: &Vec3) -> Option<(f32, f32)> {
        let (g, v) = (self.info.gravity, self.speed);
        let delta = *to - *from;
        let dist = delta.length_2d();
        if let Some(arcs) = &self.arcs {
            return arcs.solve(dist, delta.z);
        }
        let pitch = if g == 0.0 {
            (-helpers::calculate_angle(from, to).x).to_radians()
        } else {
            let root = v.powi(4) - g * (g * dist * dist + 2.0 * delta.z * v * v);
            if root < 0.0 {
                return None;
            }
            ((v * v - root.sqrt()) / (g * dist)).atan()
        };
        Some((pitch, dist / (pitch.cos() * v)))
    }

    /// CAimbotProjectile::CalculateAngle, the angles that land on `target` and whether the
    /// projectile gets there within `ticks`
    fn solve(&self, target: &Vec3, ticks: i32) -> Solve {
        let tick = Interfaces::global_vars().interval_per_tick;
        let Some((pitch, time)) = self.trajectory(&self.eye, target) else {
            return Solve::Bad;
        };
        let angle_to = helpers::calculate_angle(&self.eye, target);
        let mut out_pitch = -pitch.to_degrees() - self.ang_fix.x;
        let mut out_yaw = angle_to.y - self.ang_fix.y;
        let mut time = time - self.offset_time;
        let mut yaw = out_yaw;
        let mut pitch_deg = out_pitch;

        if (time / tick) as i32 + 1 > ticks {
            return Solve::Late;
        }

        // Again from where the projectile actually starts and heads
        let (pos, ang) = fire_setup(
            self.localplayer,
            &self.eye,
            &Vec3::new(pitch_deg, yaw, 0.0),
            &self.info,
        );
        let Some((proj_pitch, proj_time)) = self.trajectory(&pos, target) else {
            return Solve::Bad;
        };
        time = proj_time;

        // Yaw: turn so the projectile's line from its start passes over the target
        {
            let shoot = (pos - self.eye).to_2d();
            let to = *target - self.eye;
            let forward = ang.angle_vectors().0.to_2d().normalized();
            let b = 2.0 * (shoot.x * forward.x + shoot.y * forward.y);
            let c = shoot.length_2d().powi(2) - to.length_2d().powi(2);
            if let Some(t) = quadratic_larger(b, c) {
                let reach = shoot + forward * t;
                let reach_yaw = reach.y.atan2(reach.x).to_degrees();
                out_yaw = yaw - (reach_yaw - yaw);
                yaw = reach_yaw;
            }
        }

        // Pitch
        if self.info.gravity != 0.0 {
            pitch_deg -= ang.x;
            out_pitch = -proj_pitch.to_degrees() + pitch_deg - self.ang_fix.x;
        } else {
            let rotate = |v: Vec3| {
                let (s, c) = (-yaw).to_radians().sin_cos();
                Vec3::new(v.x * c - v.y * s, v.x * s + v.y * c, v.z)
            };
            let mut shoot = rotate(pos - self.eye);
            shoot.y = 0.0;
            let to = rotate(*target - self.eye);
            let mut forward = (ang - Vec3::new(0.0, yaw, 0.0)).angle_vectors().0;
            forward.y = 0.0;
            let forward = forward.normalized();
            let b = 2.0 * (shoot.x * forward.x + shoot.z * forward.z);
            let c = (shoot.x * shoot.x + shoot.z * shoot.z) - (to.x * to.x + to.z * to.z);
            if let Some(t) = quadratic_larger(b, c) {
                let reach = shoot + forward * t;
                out_pitch = pitch_deg - ((-reach.z).atan2(reach.x).to_degrees() - pitch_deg);
            }
        }

        if (time / tick) as i32 + 1 > ticks {
            return Solve::Late;
        }
        Solve::Good(Solution {
            angles: Vec3::new(out_pitch, out_yaw, 0.0),
            time,
        })
    }

    /// Positions of the projectile fired along `angles`, every tick for `time` seconds
    fn path(&self, angles: &Vec3, time: f32) -> Vec<Vec3> {
        let tick = Interfaces::global_vars().interval_per_tick;
        let (pos, ang) = fire_setup(self.localplayer, &self.eye, angles, &self.info);
        let v = launch_velocity(&ang, &self.info);
        let mut out = Vec::new();
        if self.info.drag > 0.0 {
            let (mut pos, mut vel) = (pos, v);
            out.push(pos);
            let mut t = 0.0;
            while t < time {
                let dt = tick.min(time - t);
                physics_step(&mut pos, &mut vel, dt, &self.info);
                out.push(pos);
                t += dt;
            }
            return out;
        }
        let mut t = 0.0;
        loop {
            let p = pos + v * t - Vec3::new(0.0, 0.0, 0.5 * self.info.gravity * t * t);
            out.push(p);
            if t >= time {
                break;
            }
            t = (t + tick).min(time);
        }
        out
    }

    /// Whether the projectile reaches `point` without running into the world first
    fn clear(&self, path: &[Vec3], point: &Vec3, tolerance: f32) -> bool {
        let hull = self.info.hull.min(3.0);
        let (mins, maxs) = (Vec3::new(-hull, -hull, -hull), Vec3::new(hull, hull, hull));
        let filter = TraceFilter::world_only();
        for w in path.windows(2) {
            let trace = Interfaces::engine_trace().trace_hull(
                &w[0],
                &w[1],
                &mins,
                &maxs,
                mask::SHOT,
                &filter,
            );
            if trace.did_hit() {
                return trace.end_pos.distance_to(point) <= tolerance;
            }
        }
        true
    }
}

/// Larger root of t² + bt + c
fn quadratic_larger(b: f32, c: f32) -> Option<f32> {
    let root = b * b - 4.0 * c;
    (root >= 0.0).then(|| (-b + root.sqrt()) / 2.0)
}

/// Points on a player to aim at, best first, relative to their origin
fn aim_points(
    target: &Player,
    info: &ProjectileInfo,
    on_ground: bool,
) -> impl Iterator<Item = Vec3> {
    let (mins, maxs) = (target.mins(), target.maxs());
    let head = Vec3::new(0.0, 0.0, maxs.z - 5.0);
    let body = Vec3::new(0.0, 0.0, (maxs.z - mins.z) / 2.0);
    let feet = Vec3::new(0.0, 0.0, mins.z + 5.0);
    if info.headshot {
        // Between the eyes and the top of the box, Amalgam's huntsman lerp
        let eye = target.view_offset().z.clamp(mins.z + 5.0, maxs.z - 5.0);
        [
            Some(Vec3::new(0.0, 0.0, eye + (maxs.z - eye) * 0.5)),
            Some(body),
            None,
        ]
    } else if info.splash && on_ground {
        [Some(feet), Some(body), None]
    } else {
        [Some(body), Some(head), Some(feet)]
    }
    .into_iter()
    .flatten()
}

/// Directions spread evenly over a sphere
fn sphere_directions() -> &'static [Vec3] {
    use once_cell::sync::OnceCell;
    static DIRS: OnceCell<Vec<Vec3>> = OnceCell::new();
    DIRS.get_or_init(|| {
        let golden = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
        (0..SPLASH_DIRECTIONS)
            .map(|i| {
                let z = 1.0 - (i as f32 + 0.5) / SPLASH_DIRECTIONS as f32 * 2.0;
                let r = (1.0 - z * z).sqrt();
                let (sin, cos) = (golden * i as f32).sin_cos();
                Vec3::new(cos * r, sin * r, z)
            })
            .collect()
    })
}

/// Spots on surfaces around a player standing at `origin` that a blast would reach them from,
/// facing `eye` and closest first
fn splash_points(origin: Vec3, target: &Player, info: &ProjectileInfo, eye: &Vec3) -> Vec<Vec3> {
    let (mins, maxs) = (target.mins(), target.maxs());
    let center = origin + Vec3::new(0.0, 0.0, (mins.z + maxs.z) / 2.0);
    // A little short of the full radius, the blast falls off and positions aren't exact
    let reach = info.splash_radius * 0.9;
    // Off the surface by the projectile's size so it doesn't clip it on the way in
    let lift = info.hull.min(3.0) + 1.0;
    let filter = TraceFilter::world_only();

    let mut points: Vec<(f32, Vec3)> = sphere_directions()
        .iter()
        .filter_map(|dir| {
            let ray = Ray::init(&center, &(center + *dir * reach));
            let trace = Interfaces::engine_trace().trace(&ray, mask::SOLID, &filter);
            if trace.start_solid || trace.fraction >= 1.0 {
                return None;
            }
            let normal = trace.plane_normal();
            let point = trace.end_pos + normal * lift;
            // The side of the surface we can hit
            (normal.dot(&(*eye - point)) > 0.0).then(|| (trace.fraction, point))
        })
        .collect();
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    points
        .into_iter()
        .take(SPLASH_CANDIDATES)
        .map(|(_, p)| p)
        .collect()
}

struct Found {
    index: i32,
    angles: Vec3,
    time: f32,
    fov: f32,
    paths: PredictionPaths,
    /// What the shot was worked out from, for the shot log
    shot: shot_log::Prediction,
}

/// Why a shot at a point was turned down
#[derive(Clone, Copy)]
pub enum Reject {
    Late,
    Bad,
    Blocked,
}

/// Total real latency, how long until the server fires our projectile plus how old what we see is
fn latency() -> f32 {
    Interfaces::engine_client()
        .get_net_channel_info()
        .map_or(0.0, |nci| {
            nci.latency(Flow::Outgoing) + nci.latency(Flow::Incoming)
        })
}

pub fn run(localplayer: &Player, weapon: &Weapon, cmd: &mut UserCmd, config: &Config) {
    track_in_flight(localplayer);
    let aim = &config.aimbot;
    let Some(info) = weapon_info(localplayer, weapon) else {
        Globals::write().target = None;
        return;
    };

    const ATTACK: i32 = Buttons::InAttack as i32;
    let attacking = (cmd.buttons & ATTACK) != 0;
    let key = aim.key.use_key;
    let key_down = Globals::read().aimbot_key_down;
    let charging = info.charge && charge_time(weapon, localplayer.curtime()) > 0.0;
    let can_attack = localplayer.can_attack();

    // Charged weapons fire when attack is let go. With the key they charge while it's held and
    // let go once there's a shot, otherwise the shot is aimed when the user lets go.
    // Others aim whenever they'd fire, even while the weapon isn't ready: attack with a loaded
    // clip cuts a reload short, and the shot leaves as soon as the game allows.
    let looking = if info.charge {
        if key && key_down {
            cmd.buttons |= ATTACK;
        }
        charging && if key { key_down } else { !attacking }
    } else if key {
        key_down
    } else {
        attacking
    };
    if !looking {
        Globals::write().target = None;
        return;
    }

    let logging = aim.log_prediction;
    let started = std::time::Instant::now();
    let (found, diagnostics) = find_target(localplayer, cmd, config, info, logging);
    if logging {
        shot_log::search_time(started.elapsed().as_secs_f32() * 1000.0);
    }
    Globals::write().target = found.as_ref().map(|f| Target {
        target_index: f.index,
        should_headshot: info.headshot,
    });
    let Some(found) = found else {
        if logging {
            shot_log::no_shot(&diagnostics);
        }
        // Hold fire until our last shot lands rather than waste one
        if !info.charge && diagnostics.iter().any(|d| d.waiting) {
            cmd.buttons &= !ATTACK;
        }
        return;
    };

    if info.charge {
        cmd.buttons &= !ATTACK;
    } else if key {
        cmd.buttons |= ATTACK;
    }
    cmd.view_angles = found.angles;

    // Charged weapons fire on the command that lets go, others on any with attack held
    let fires = if info.charge {
        charging && (cmd.buttons & ATTACK) == 0
    } else {
        can_attack && (cmd.buttons & ATTACK) != 0
    };
    // Out of ammo the attack does nothing, and mid reload the first press only cuts it short
    let fires = fires && weapon.clip() != 0;
    let tick = Interfaces::global_vars().interval_per_tick;
    let s = &found.shot;
    if fires && info.splash && !info.charge {
        TRACKING.lock().unwrap().in_flight.push(InFlight {
            target: found.index,
            land_time: s.start_time + (s.choke + s.latency_ticks + s.ticks) as f32 * tick,
            fired: Interfaces::global_vars().realtime,
            projectile: None,
            expires: Interfaces::global_vars().realtime + found.time + latency() + 0.1,
        });
    }
    if logging && fires {
        shot_log::shot(
            localplayer,
            &found.shot,
            &found.paths.projectile,
            found.time,
            found.fov,
        );
    }

    // Only what's fired, a line for a shot that never left would look like a miss
    if aim.draw_prediction && fires {
        let now = Interfaces::global_vars().realtime;
        let mut paths = found.paths;
        paths.until = now + found.time + latency();
        Globals::write().prediction = Some(paths);
    }
}

fn find_target(
    localplayer: &Player,
    cmd: &UserCmd,
    config: &Config,
    info: ProjectileInfo,
    logging: bool,
) -> (Option<Found>, Vec<shot_log::TargetDiagnostics>) {
    let aim = &config.aimbot;
    let mut diagnostics = Vec::new();
    let tick = Interfaces::global_vars().interval_per_tick;
    let eye = localplayer.eye_pos();
    let strafe = Strafe {
        ground: aim.strafe_ground,
        air: aim.strafe_air,
    };
    let latency_ticks = (latency() / tick).round() as i32;
    let max_ticks = ((aim.max_sim_time as f32 / 1000.0).min(info.lifetime) / tick) as i32;
    let solver = Solver::new(localplayer, eye, info, max_ticks as f32 * tick);

    // Closest to the crosshair first, simulating everyone would be too slow
    let mut targets: Vec<(f32, Player)> = (1..=Interfaces::engine_client().get_max_clients())
        .filter_map(|i| Interfaces::entity_list().get_client_entity::<Player>(i))
        .filter(|p| {
            *p != *localplayer
                && !p.is_dormant()
                && !p.is_dead()
                && p.team() != localplayer.team()
                && !(aim.ignore_invis && p.is_invisible())
        })
        .map(|p| {
            let center = p.origin() + Vec3::new(0.0, 0.0, p.maxs().z / 2.0);
            (
                cmd.view_angles
                    .fov_to(&helpers::calculate_angle(&eye, &center)),
                p,
            )
        })
        .filter(|(fov, _)| *fov <= aim.fov as f32)
        .collect();
    targets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    targets.truncate(MAX_TARGETS);

    let use_splash = aim.splash && info.splash_radius > 0.0;
    let splash_slack = info.splash_radius / solver.speed;

    // Read once rather than for each target
    let landings = if aim.wait_for_shot {
        landings()
    } else {
        Vec::new()
    };

    let mut best: Option<Found> = None;
    for (center_fov, target) in targets {
        let mut diag = shot_log::TargetDiagnostics {
            index: target.index(),
            fov: center_fov,
            distance: target.origin().distance_to(&eye),
            on_ground: target.is_on_ground(),
            ..Default::default()
        };
        let Some(mut sim) = MoveSim::new(&target, None, strafe) else {
            diag.no_sim = true;
            diagnostics.push(diag);
            continue;
        };
        let start_time = target.simulation_time();
        let choke = sim.choke();
        let wait_for = landings
            .iter()
            .filter(|(t, _)| *t == target.index())
            .map(|(_, land_time)| *land_time)
            .min_by(f32::total_cmp);
        let mut waiting = false;
        let mut count = |r: Result<_, Reject>| match r {
            Ok(v) => Some(v),
            Err(reject) => {
                if logging {
                    diag.count(reject);
                }
                None
            }
        };
        // The projectile leaves once the server gets the command, and what we see is behind
        for _ in 0..latency_ticks {
            sim.run_tick();
        }

        let mut splash_ticks = 0;
        for ticks in 1..=max_ticks {
            let land_time = start_time + (choke + latency_ticks + ticks) as f32 * tick;
            if wait_for.is_some_and(|w| land_time >= w) {
                waiting = true;
                break;
            }
            sim.run_tick();
            let origin = sim.predicted_origin();
            let direct_tolerance = info.hull.max(4.0) + 8.0;
            let mut hit = aim_points(&target, &info, sim.on_ground())
                .enumerate()
                .find_map(|(n, offset)| {
                    let extra = if n == 0 && info.splash { 8.0 } else { 0.0 };
                    let point = origin + offset;
                    count(attempt(
                        &solver,
                        cmd,
                        &point,
                        ticks,
                        direct_tolerance + extra,
                    ))
                    .map(|hit| (hit, point, false))
                });

            // Spots near the target are at most the blast radius closer than it, so there's
            // no point looking before the projectile could nearly make it to the target
            if hit.is_none() && use_splash && splash_ticks < SPLASH_TICKS {
                let center = origin + Vec3::new(0.0, 0.0, target.maxs().z / 2.0);
                let in_window = solver
                    .trajectory(&eye, &center)
                    .is_some_and(|(_, time)| time - splash_slack <= ticks as f32 * tick);
                if in_window {
                    splash_ticks += 1;
                    let tolerance = info.hull.max(4.0) + 12.0;
                    hit = splash_points(origin, &target, &info, &eye)
                        .iter()
                        .find_map(|point| {
                            count(attempt(&solver, cmd, point, ticks, tolerance))
                                .map(|hit| (hit, *point, true))
                        });
                }
            }

            let Some(((solution, path, fov), point, splash)) = hit else {
                continue;
            };
            if best.as_ref().is_none_or(|b| fov < b.fov) {
                let mut player_path = sim.path.clone();
                player_path.push(sim.origin());
                best = Some(Found {
                    index: target.index(),
                    angles: solution.angles,
                    time: solution.time,
                    fov,
                    paths: PredictionPaths {
                        player: player_path,
                        projectile: path,
                        until: 0.0,
                    },
                    shot: shot_log::Prediction {
                        target: target.index(),
                        start_time,
                        choke,
                        latency_ticks,
                        ticks,
                        origin,
                        point,
                        splash,
                        target_on_ground: sim.on_ground(),
                    },
                });
            }
            break;
        }
        drop(count);
        diag.waiting = waiting;
        diagnostics.push(diag);
    }
    (best, diagnostics)
}

/// A shot landing on `point` within `ticks` that gets there unobstructed
fn attempt(
    solver: &Solver,
    cmd: &UserCmd,
    point: &Vec3,
    ticks: i32,
    tolerance: f32,
) -> Result<(Solution, Vec<Vec3>, f32), Reject> {
    let solution = match solver.solve(point, ticks) {
        Solve::Good(s) => s,
        Solve::Late => return Err(Reject::Late),
        Solve::Bad => return Err(Reject::Bad),
    };
    let fov = cmd.view_angles.fov_to(&solution.angles);
    let path = solver.path(&solution.angles, solution.time);
    if !solver.clear(&path, point, tolerance) {
        return Err(Reject::Blocked);
    }
    Ok((solution, path, fov))
}
