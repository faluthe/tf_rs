//! Projectile aimbot.
//!
//! Targets are moved forward tick by tick with movement simulation. At each tick the angle that
//! lands a projectile on them is solved for, and the first tick the projectile gets there in time
//! is the one aimed at. The solve accounts for the projectile starting off to the side of the eye
//! and, for rockets and arrows, being angled in towards the crosshair.

use std::cmp::Ordering;

use crate::{
    config::Config,
    features::movesim::{self, MoveSim, Strafe},
    globals::{Globals, PredictionPaths, Target},
    helpers,
    interfaces::Interfaces,
    types::weapon::WeaponClass,
    types::{Flow, Player, TraceFilter, UserCmd, Vec3, Weapon, user_cmd::Buttons},
};

const MASK_SOLID: u32 = 0x200400B;
const MASK_SHOT: u32 = 0x4600400B;
const MAX_TARGETS: usize = 3;

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
    /// Half size of the projectile's box
    hull: f32,
    /// Seconds before it's gone, the simulation stops there
    lifetime: f32,
    /// Explodes, so aim at the feet of players on the ground
    splash: bool,
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
        hull: 0.0,
        lifetime: f32::MAX,
        splash: false,
        headshot: false,
        charge: false,
    };

    match weapon.weapon_class() {
        WeaponClass::Rocketlauncher | WeaponClass::RocketlauncherDirecthit => {
            let center = attrib(0.0, c"centerfire_projectile") as i32 == 1;
            info.offset.y = if center { 0.0 } else { 12.0 };
            info.speed = attrib(1100.0, c"mult_projectile_speed");
            info.splash = true;
        }
        WeaponClass::ParticleCannon | WeaponClass::Raygun | WeaponClass::DrgPomson => {
            let mangler = matches!(weapon.weapon_class(), WeaponClass::ParticleCannon);
            if matches!(weapon.weapon_class(), WeaponClass::DrgPomson) {
                info.offset.z -= 13.0;
            }
            info.speed = if mangler { 1100.0 } else { 1200.0 };
            info.hull = if mangler { 0.0 } else { 1.0 };
            info.splash = mangler;
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
            info.hull = 6.0;
            info.splash = true;
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
    let trace = Interfaces::engine_trace().trace_ray(eye, &end, MASK_SOLID, Some(localplayer));
    if trace.did_hit() && trace.fraction > 0.1 {
        end = trace.end_pos;
    }
    (pos, (end - pos).to_angles())
}

/// Launch velocity for a projectile heading along `angles`
fn launch_velocity(angles: &Vec3, info: &ProjectileInfo) -> Vec3 {
    let (forward, _, up) = angles.angle_vectors();
    forward * info.speed + up * info.up_speed
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
    fn new<'a>(localplayer: &'a Player, eye: Vec3, info: ProjectileInfo) -> Solver<'a> {
        let v = launch_velocity(&Vec3::default(), &info);
        let (pos, _) = fire_setup(localplayer, &eye, &Vec3::default(), &info);
        let speed = v.length();
        Solver {
            localplayer,
            eye,
            info,
            speed,
            ang_fix: v.to_angles(),
            offset_time: (pos - eye).length() / speed,
        }
    }

    /// Launch pitch (radians, up positive) and flight time from `from` to `to`
    fn trajectory(&self, from: &Vec3, to: &Vec3) -> Option<(f32, f32)> {
        let (g, v) = (self.info.gravity, self.speed);
        let delta = *to - *from;
        let dist = delta.length_2d();
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
            let trace = Interfaces::engine_trace()
                .trace_hull(&w[0], &w[1], &mins, &maxs, MASK_SHOT, &filter);
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
fn aim_points(target: &Player, info: &ProjectileInfo, on_ground: bool) -> Vec<Vec3> {
    let (mins, maxs) = (target.mins(), target.maxs());
    let head = Vec3::new(0.0, 0.0, maxs.z - 5.0);
    let body = Vec3::new(0.0, 0.0, (maxs.z - mins.z) / 2.0);
    let feet = Vec3::new(0.0, 0.0, mins.z + 5.0);
    if info.headshot {
        // Between the eyes and the top of the box, Amalgam's huntsman lerp
        let eye = target.view_offset().z.clamp(mins.z + 5.0, maxs.z - 5.0);
        return vec![Vec3::new(0.0, 0.0, eye + (maxs.z - eye) * 0.5), body];
    }
    if info.splash && on_ground {
        return vec![feet, body];
    }
    vec![body, head, feet]
}

struct Found {
    index: i32,
    angles: Vec3,
    time: f32,
    fov: f32,
    paths: PredictionPaths,
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
    let looking = if info.charge {
        if key && key_down {
            cmd.buttons |= ATTACK;
        }
        charging && if key { key_down } else { !attacking }
    } else if key {
        key_down && can_attack
    } else {
        attacking && can_attack
    };
    if !looking {
        Globals::write().target = None;
        return;
    }

    let found = find_target(localplayer, cmd, config, info);
    Globals::write().target = found.as_ref().map(|f| Target {
        target_index: f.index,
        should_headshot: info.headshot,
    });
    let Some(found) = found else {
        return;
    };

    if info.charge {
        cmd.buttons &= !ATTACK;
    } else if key {
        cmd.buttons |= ATTACK;
    }
    cmd.view_angles = found.angles;

    if aim.draw_prediction {
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
) -> Option<Found> {
    let aim = &config.aimbot;
    let tick = Interfaces::global_vars().interval_per_tick;
    let eye = localplayer.eye_pos();
    let solver = Solver::new(localplayer, eye, info);
    let strafe = Strafe {
        ground: aim.strafe_ground,
        air: aim.strafe_air,
    };
    let latency_ticks = (latency() / tick).round() as i32;
    let max_ticks = ((aim.max_sim_time as f32 / 1000.0).min(info.lifetime) / tick) as i32;

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
        .filter(|(fov, _)| *fov <= aim.fov as f32 * 1.5 + 10.0)
        .collect();
    targets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    targets.truncate(MAX_TARGETS);

    let mut best: Option<Found> = None;
    for (_, target) in targets {
        let Some(mut sim) = MoveSim::new(&target, None, strafe) else {
            continue;
        };
        // The projectile leaves once the server gets the command, and what we see is behind
        for _ in 0..latency_ticks {
            sim.run_tick();
        }

        'ticks: for ticks in 1..=max_ticks {
            sim.run_tick();
            let origin = sim.predicted_origin();
            let points = aim_points(&target, &info, sim.on_ground());
            for (n, offset) in points.iter().enumerate() {
                let point = origin + *offset;
                let solution = match solver.solve(&point, ticks) {
                    Solve::Good(s) => s,
                    Solve::Late => continue,
                    Solve::Bad => continue,
                };
                let fov = cmd.view_angles.fov_to(&solution.angles);
                if fov > aim.fov as f32 {
                    continue;
                }
                let path = solver.path(&solution.angles, solution.time);
                let tolerance = info.hull.max(4.0) + if n == 0 && info.splash { 16.0 } else { 8.0 };
                if !solver.clear(&path, &point, tolerance) {
                    continue;
                }
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
                    });
                }
                break 'ticks;
            }
        }
    }
    best
}
