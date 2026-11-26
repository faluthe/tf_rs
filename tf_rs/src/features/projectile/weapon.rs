//! How each weapon's projectile leaves it

use crate::{
    features::movesim,
    interfaces::{Interfaces, mask},
    types::{Player, Ray, TraceFilter, Vec3, Weapon, trace_filter::group, weapon::WeaponClass},
};

/// How the launch point is pulled back towards the eye when there's a wall in the way
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Clip {
    /// It isn't, flares and arrows start where they're put
    None,
    /// To where a line from the eye hits a brush, rockets and energy shots
    Line,
    /// To where an 8 unit box from the eye hits a brush, pipes. Starting stuck stops the shot.
    Hull,
}

/// How a weapon's projectile leaves it
#[derive(Clone, Copy)]
pub(super) struct ProjectileInfo {
    /// From the eye, forward/right/up
    pub(super) offset: Vec3,
    /// Fired straight along the view instead of angled in to meet the crosshair
    pub(super) pipes: bool,
    pub(super) clip: Clip,
    /// Drop applied to the launch point after the angle is solved, the Pomson's muzzle
    pub(super) drop: f32,
    pub(super) speed: f32,
    /// Extra upwards launch speed
    pub(super) up_speed: f32,
    pub(super) gravity: f32,
    /// Fraction of speed lost per second per unit of speed, fitted to logged flights
    pub(super) drag: f32,
    /// Half size of the projectile's box
    pub(super) hull: f32,
    /// Collision group
    pub(super) group: i32,
    /// Runs into enemy players. Pipes, Bison and Pomson pass through and hit by touching.
    pub(super) hits_players: bool,
    /// Seconds into the flight it starts running into teammates too
    pub(super) teammates_after: f32,
    /// Seconds before it's gone
    pub(super) lifetime: f32,
    /// Explodes, so aim at the feet of players on the ground
    pub(super) splash: bool,
    pub(super) splash_radius: f32,
    pub(super) headshot: bool,
    pub(super) charge: bool,
}

pub(super) fn charge_time(weapon: &Weapon, now: f32) -> f32 {
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

/// How the held weapon fires its projectile
pub(super) fn weapon_info(localplayer: &Player, weapon: &Weapon) -> Option<ProjectileInfo> {
    let gravity = movesim::gravity();
    let ducking = localplayer.is_ducking();
    let now = localplayer.curtime();
    let attrib = |value: f32, name: &std::ffi::CStr| weapon.attrib_float(value, name);
    let mut info = ProjectileInfo {
        offset: Vec3::new(23.5, 8.0, if ducking { 8.0 } else { -3.0 }),
        pipes: false,
        clip: Clip::None,
        drop: 0.0,
        speed: 0.0,
        up_speed: 0.0,
        gravity: 0.0,
        drag: 0.0,
        hull: 0.0,
        // CTFBaseRocket, passes through teammates for its first quarter second
        group: group::ROCKETS,
        hits_players: true,
        teammates_after: 0.25,
        lifetime: f32::MAX,
        splash: false,
        splash_radius: 0.0,
        headshot: false,
        charge: false,
    };

    let class = weapon.weapon_class();
    match class {
        WeaponClass::Rocketlauncher | WeaponClass::RocketlauncherDirecthit => {
            let center = attrib(0.0, c"centerfire_projectile") as i32 == 1;
            info.offset.y = if center { 0.0 } else { 12.0 };
            info.speed = attrib(1100.0, c"mult_projectile_speed");
            info.clip = Clip::Line;
            info.splash = true;
            info.splash_radius = attrib(146.0, c"mult_explosion_radius");
        }
        WeaponClass::ParticleCannon | WeaponClass::Raygun | WeaponClass::DrgPomson => {
            let mangler = matches!(class, WeaponClass::ParticleCannon);
            if matches!(class, WeaponClass::DrgPomson) {
                info.drop = 13.0;
            }
            info.speed = if mangler { 1100.0 } else { 1200.0 };
            info.hull = if mangler { 0.0 } else { 1.0 };
            info.hits_players = mangler;
            info.clip = Clip::Line;
            info.splash = mangler;
            info.splash_radius = if mangler { 146.0 } else { 0.0 };
        }
        WeaponClass::Grenadelauncher | WeaponClass::Cannon => {
            info.offset = Vec3::new(16.0, 8.0, -6.0);
            info.pipes = true;
            info.clip = Clip::Hull;
            info.speed = attrib(
                attrib(1200.0, c"mult_projectile_speed"),
                c"mult_projectile_range",
            );
            info.up_speed = 200.0;
            info.gravity = 800.0;
            info.drag = if matches!(class, WeaponClass::Cannon) {
                0.385 / 1454.0
            } else if attrib(0.0, c"grenade_no_spin") != 0.0 {
                // Loch-n-Load, doesn't spin
                0.175 / 1504.0
            } else {
                0.12 / 1217.0
            };
            info.hull = 6.0;
            info.group = group::GRENADES;
            info.hits_players = false;
            let fuse = attrib(2.0, c"fuse_mult");
            info.lifetime = ((fuse / 0.195).floor() + 1.0) * 0.195;
        }
        WeaponClass::Pipebomblauncher => {
            info.offset = Vec3::new(16.0, 8.0, -6.0);
            info.pipes = true;
            info.clip = Clip::Hull;
            let rate = attrib(4.0, c"stickybomb_charge_rate");
            let speed = remap(charge_time(weapon, now), (0.0, rate), (900.0, 2400.0));
            info.speed = attrib(speed, c"mult_projectile_range");
            info.up_speed = 200.0;
            info.gravity = 800.0;
            info.drag = 0.085 / 922.0;
            info.hull = 6.0;
            info.group = group::GRENADES;
            info.hits_players = false;
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
            info.teammates_after = f32::MAX;
            info.headshot = true;
            info.charge = true;
        }
        WeaponClass::Crossbow | WeaponClass::ShotgunBuildingRescue => {
            info.offset = Vec3::new(23.5, 8.0, -3.0);
            info.speed = 2400.0;
            info.gravity = 0.2 * gravity;
            info.hull = 1.0;
            info.teammates_after = f32::MAX;
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
pub(super) fn fire_setup(
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
    let filter = TraceFilter::ignore_teammates(localplayer);
    let trace = Interfaces::engine_trace().trace(&Ray::init(eye, &end), mask::SOLID, &filter);
    if trace.did_hit() && trace.fraction > 0.1 {
        end = trace.end_pos;
    }
    let angles = (end - pos).to_angles();
    (pos - Vec3::new(0.0, 0.0, info.drop), angles)
}

pub(super) fn launch_velocity(angles: &Vec3, info: &ProjectileInfo) -> Vec3 {
    let (forward, _, up) = angles.angle_vectors();
    let v = forward * info.speed + up * info.up_speed;
    if info.pipes && v.length() > MAX_PHYSICS_SPEED {
        return v.normalized() * MAX_PHYSICS_SPEED;
    }
    v
}

const MAX_PHYSICS_SPEED: f32 = 2000.0;
