//! Flights of projectiles with drag

use std::sync::{Arc, Mutex};

use super::weapon::ProjectileInfo;
use crate::{interfaces::Interfaces, types::Vec3};

/// Physics steps a physics projectile has already taken on the tick it's fired
pub(super) const PHYSICS_HEAD_START: usize = 1;

/// One tick of a physics projectile's flight: gravity and drag change its velocity, then it moves
pub(super) fn physics_step(pos: &mut Vec3, vel: &mut Vec3, dt: f32, info: &ProjectileInfo) {
    vel.z -= info.gravity * dt;
    let slow = (info.drag * vel.length() * dt).min(1.0);
    *vel = *vel * (1.0 - slow);
    *pos += *vel * dt;
}

/// Distance and height every tick of a physics projectile launched at each whole degree of pitch.
/// Drag leaves no closed form for the arc, so it's solved against this.
pub(super) struct ArcTable {
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
                for _ in 0..PHYSICS_HEAD_START {
                    physics_step(&mut pos, &mut vel, tick, info);
                }
                arc.push((pos.x, pos.z));
                for _ in 0..ticks {
                    physics_step(&mut pos, &mut vel, tick, info);
                    arc.push((pos.x, pos.z));
                }
                arc
            })
            .collect();
        ArcTable { pitches, arcs }
    }

    /// The last table built if nothing changed, building one every command is too slow
    pub(super) fn cached(info: &ProjectileInfo, speed: f32, max_time: f32) -> Arc<Self> {
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
    pub(super) fn solve(&self, distance: f32, height: f32) -> Option<(f32, f32)> {
        let tick = Interfaces::global_vars().interval_per_tick;
        // Height at a distance rises with pitch up to the furthest arc, so the first pitch high
        // enough is the low arc
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
