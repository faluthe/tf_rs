//! Where on and around a target to aim

use std::cmp::Ordering;

use super::{count_trace, weapon::ProjectileInfo};
use crate::{
    interfaces::{Interfaces, mask},
    types::{Player, Ray, TraceFilter, Vec3},
};

/// Surface directions looked along around a target for splash
const SPLASH_DIRECTIONS: usize = 32;
/// Closest splash spots tried per tick
const SPLASH_CANDIDATES: usize = 6;

/// Points on a player to aim at, best first, relative to their origin
pub(super) fn aim_points(
    target: &Player,
    info: &ProjectileInfo,
    on_ground: bool,
    head_offset: Option<Vec3>,
) -> impl Iterator<Item = Vec3> {
    let (mins, maxs) = (target.mins(), target.maxs());
    let head = Vec3::new(0.0, 0.0, maxs.z - 5.0);
    let body = Vec3::new(0.0, 0.0, (maxs.z - mins.z) / 2.0);
    let feet = Vec3::new(0.0, 0.0, mins.z + 5.0);
    if info.headshot {
        // The head hitbox, or between the eyes and the top of the box without one
        let head = head_offset.unwrap_or_else(|| {
            let eye = target.view_offset().z.clamp(mins.z + 5.0, maxs.z - 5.0);
            Vec3::new(0.0, 0.0, eye + (maxs.z - eye) * 0.5)
        });
        [Some(head), Some(body), None]
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
pub(super) fn splash_points(
    shooter: &Player,
    origin: Vec3,
    target: &Player,
    info: &ProjectileInfo,
    eye: &Vec3,
) -> Vec<Vec3> {
    let (mins, maxs) = (target.mins(), target.maxs());
    let center = origin + Vec3::new(0.0, 0.0, (mins.z + maxs.z) / 2.0);
    // A little short of the full radius, the blast falls off and positions aren't exact
    let reach = info.splash_radius * 0.9;
    // Off the surface by the projectile's size so it doesn't clip it on the way in
    let lift = info.hull + 1.0;
    let filter = TraceFilter::simple(shooter, info.group, Some(target));

    let mut points: Vec<(f32, Vec3)> = sphere_directions()
        .iter()
        .filter_map(|dir| {
            let ray = Ray::init(&center, &(center + *dir * reach));
            count_trace();
            let trace = Interfaces::engine_trace().trace(&ray, mask::SOLID, &filter);
            if trace.start_solid || trace.fraction >= 1.0 {
                return None;
            }
            let normal = trace.plane_normal();
            let point = trace.end_pos + normal * lift;
            // The side of the surface we can hit
            (normal.dot(&(*eye - point)) > 0.0).then_some((trace.fraction, point))
        })
        .collect();
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    points
        .into_iter()
        .take(SPLASH_CANDIDATES)
        .map(|(_, p)| p)
        .collect()
}
