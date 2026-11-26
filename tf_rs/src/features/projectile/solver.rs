//! Aim angles that land a projectile on a point, and whether its flight gets there

use std::sync::Arc;

use super::{
    Reject,
    arcs::{ArcTable, PHYSICS_HEAD_START, physics_step},
    count_trace,
    weapon::{Clip, ProjectileInfo, fire_setup, launch_velocity},
};
use crate::{
    helpers,
    interfaces::{Interfaces, mask},
    types::{Entity, Hull, Player, TraceFilter, Vec3, entity::Team, trace_filter::group},
};

/// How many ticks early the projectile can reach the target's box and still hit
const EARLY_TICKS: i32 = 5;
/// Ticks of an arcing flight traced at once, short enough to stray under a unit from the arc
const TRACE_STRIDE: usize = 5;

pub(super) struct Solver<'a> {
    pub(super) localplayer: &'a Player,
    pub(super) eye: Vec3,
    pub(super) info: ProjectileInfo,
    /// Launch speed including the upwards part
    pub(super) speed: f32,
    /// Angle the launch velocity makes with the view
    ang_fix: Vec3,
    offset_time: f32,
    /// For projectiles with drag
    arcs: Option<Arc<ArcTable>>,
    /// A wall is close enough to the eye to pull launch points back
    near_wall: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Solution {
    pub(super) angles: Vec3,
    pub(super) time: f32,
}

pub(super) enum Solve {
    Good(Solution),
    /// The projectile gets there after the target does
    Late,
    /// Out of reach
    Bad,
}

impl Solver<'_> {
    /// A solver for flights up to `max_time` long
    pub(super) fn new<'a>(
        localplayer: &'a Player,
        eye: Vec3,
        info: ProjectileInfo,
        max_time: f32,
    ) -> Solver<'a> {
        let v = launch_velocity(&Vec3::default(), &info);
        let (pos, _) = fire_setup(localplayer, &eye, &Vec3::default(), &info);
        let speed = v.length();
        // Nothing to clip launch points against if there are no brushes anywhere they could be
        let near_wall = info.clip != Clip::None && {
            let r = (pos - eye).length() + info.drop + 8.0;
            let trace = Interfaces::engine_trace().trace_hull(
                &eye,
                &eye,
                &Vec3::new(-r, -r, -r),
                &Vec3::new(r, r, r),
                mask::SOLID_BRUSH_ONLY,
                &TraceFilter::simple(localplayer, group::NONE, None),
            );
            trace.start_solid || trace.did_hit()
        };
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
            near_wall,
        }
    }

    /// A solver only for flight paths, without what aiming needs
    pub(super) fn for_path<'a>(
        localplayer: &'a Player,
        eye: Vec3,
        info: ProjectileInfo,
    ) -> Solver<'a> {
        let mut info = info;
        // Arc tables only matter for solving, and charged speeds would rebuild them every frame
        let drag = std::mem::replace(&mut info.drag, 0.0);
        let mut solver = Solver::new(localplayer, eye, info, 0.0);
        solver.info.drag = drag;
        solver
    }

    /// Launch pitch (radians, up positive) and flight time from `from` to `to`
    pub(super) fn trajectory(&self, from: &Vec3, to: &Vec3) -> Option<(f32, f32)> {
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
    pub(super) fn solve(&self, target: &Vec3, ticks: i32) -> Solve {
        let tick = Interfaces::global_vars().interval_per_tick;
        let late = |time: f32| (time / tick) as i32 + 1 > ticks;

        // A first guess as if the projectile left from the eye
        let Some((pitch, time)) = self.trajectory(&self.eye, target) else {
            return Solve::Bad;
        };
        if late(time - self.offset_time) {
            return Solve::Late;
        }
        let guess = Vec3::new(
            -pitch.to_degrees() - self.ang_fix.x,
            helpers::calculate_angle(&self.eye, target).y - self.ang_fix.y,
            0.0,
        );

        // Again from where the projectile really starts and heads when fired along the guess
        let Some((pos, heading)) = self.fire(&guess) else {
            return Solve::Bad;
        };
        let Some((launch_pitch, time)) = self.trajectory(&pos, target) else {
            return Solve::Bad;
        };
        if late(time) {
            return Solve::Late;
        }
        let (yaw, reach_yaw) = self.correct_yaw(&pos, &heading, target, guess.y);
        let pitch = if self.info.gravity != 0.0 {
            // The arc from where it starts, less the tilt its heading already has from the view
            -launch_pitch.to_degrees() + guess.x - heading.x - self.ang_fix.x
        } else {
            self.correct_pitch(&pos, &heading, target, guess.x, reach_yaw)
        };
        Solve::Good(Solution {
            angles: Vec3::new(pitch, yaw, 0.0),
            time,
        })
    }

    /// Yaw that turns the projectile's line from `pos` along `heading` to pass over `target`,
    /// from aiming at `yaw`. Also the yaw that line reaches the target's distance at.
    fn correct_yaw(&self, pos: &Vec3, heading: &Vec3, target: &Vec3, yaw: f32) -> (f32, f32) {
        let shoot = (*pos - self.eye).to_2d();
        let to = *target - self.eye;
        let forward = heading.angle_vectors().0.to_2d().normalized();
        let b = 2.0 * (shoot.x * forward.x + shoot.y * forward.y);
        let c = shoot.length_2d().powi(2) - to.length_2d().powi(2);
        let Some(t) = quadratic_larger(b, c) else {
            return (yaw, yaw);
        };
        let reach = shoot + forward * t;
        let reach_yaw = reach.y.atan2(reach.x).to_degrees();
        // It lands off by the difference, turn the same amount the other way
        (yaw - (reach_yaw - yaw), reach_yaw)
    }

    /// The same for pitch with projectiles that fly straight, looking along `yaw`
    fn correct_pitch(
        &self,
        pos: &Vec3,
        heading: &Vec3,
        target: &Vec3,
        pitch: f32,
        yaw: f32,
    ) -> f32 {
        // Into the vertical plane along `yaw`
        let rotate = |v: Vec3| {
            let (s, c) = (-yaw).to_radians().sin_cos();
            Vec3::new(v.x * c - v.y * s, v.x * s + v.y * c, v.z)
        };
        let mut shoot = rotate(*pos - self.eye);
        shoot.y = 0.0;
        let to = rotate(*target - self.eye);
        let mut forward = (*heading - Vec3::new(0.0, yaw, 0.0)).angle_vectors().0;
        forward.y = 0.0;
        let forward = forward.normalized();
        let b = 2.0 * (shoot.x * forward.x + shoot.z * forward.z);
        let c = (shoot.x * shoot.x + shoot.z * shoot.z) - (to.x * to.x + to.z * to.z);
        let Some(t) = quadratic_larger(b, c) else {
            return pitch;
        };
        let reach = shoot + forward * t;
        pitch - ((-reach.z).atan2(reach.x).to_degrees() - pitch)
    }

    /// Where the projectile fired along `angles` starts and which way it heads, pulled back out
    /// of any wall in the way. None when the game wouldn't fire it.
    fn fire(&self, angles: &Vec3) -> Option<(Vec3, Vec3)> {
        let (pos, heading) = fire_setup(self.localplayer, &self.eye, angles, &self.info);
        if !self.near_wall {
            return Some((pos, heading));
        }
        let h = if self.info.clip == Clip::Hull {
            8.0
        } else {
            0.0
        };
        count_trace();
        let trace = Interfaces::engine_trace().trace_hull(
            &self.eye,
            &pos,
            &Vec3::new(-h, -h, -h),
            &Vec3::new(h, h, h),
            mask::SOLID_BRUSH_ONLY,
            &TraceFilter::simple(self.localplayer, group::NONE, None),
        );
        if self.info.clip == Clip::Hull && trace.start_solid {
            return None;
        }
        Some((trace.end_pos, heading))
    }

    /// Where the projectile fired along `angles` starts and how fast it's going
    pub(super) fn launch(&self, angles: &Vec3) -> Option<(Vec3, Vec3)> {
        let (pos, ang) = self.fire(angles)?;
        Some((pos, launch_velocity(&ang, &self.info)))
    }

    /// Positions of the projectile fired along `angles`, every tick for `time` seconds. None
    /// when the game wouldn't fire it.
    pub(super) fn path(&self, angles: &Vec3, time: f32) -> Option<Vec<Vec3>> {
        let tick = Interfaces::global_vars().interval_per_tick;
        let (pos, ang) = self.fire(angles)?;
        let v = launch_velocity(&ang, &self.info);
        let mut out = Vec::new();
        if self.info.drag > 0.0 {
            let (mut pos, mut vel) = (pos, v);
            for _ in 0..PHYSICS_HEAD_START {
                physics_step(&mut pos, &mut vel, tick, &self.info);
            }
            out.push(pos);
            let mut t = 0.0;
            while t < time {
                let dt = tick.min(time - t);
                physics_step(&mut pos, &mut vel, dt, &self.info);
                out.push(pos);
                t += dt;
            }
            return Some(out);
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
        Some(out)
    }

    /// Whether `path` hits `target`'s predicted box before the world and not too early.
    /// Explosives hitting the world within `tolerance` of `point` count. On a hit, `path` is cut
    /// where it lands.
    pub(super) fn hits(
        &self,
        path: &mut Vec<Vec3>,
        target: &Entity,
        predicted: &Hull,
        point: &Vec3,
        tolerance: f32,
        ticks: i32,
    ) -> Result<(), Reject> {
        let (hull_mins, hull_maxs) = self.hull();
        let (mins, maxs) = (
            predicted.origin + predicted.mins,
            predicted.origin + predicted.maxs,
        );
        let filter = self.filter(target);
        for (a, b) in self.segments(path.len()) {
            let (from, to) = (path[a], path[b]);
            count_trace();
            let trace = Interfaces::engine_trace().trace_hull(
                &from,
                &to,
                &hull_mins,
                &hull_maxs,
                self.mask(a),
                &filter,
            );
            let world = trace.did_hit().then_some(trace.fraction);
            let entry = helpers::segment_box_entry(&from, &to, &mins, &maxs, self.info.hull)
                .filter(|t| world.is_none_or(|f| *t <= f));
            if let Some(t) = entry {
                let tick = a as f32 + t * (b - a) as f32;
                if ticks as f32 - tick.ceil() >= EARLY_TICKS as f32 {
                    return Err(Reject::Miss);
                }
                path.truncate(a + 1);
                path.push(from + (to - from) * t);
                return Ok(());
            }
            if world.is_some() {
                if !self.info.splash || trace.end_pos.distance_to(point) > tolerance {
                    return Err(Reject::Blocked);
                }
                path.truncate(a + 1);
                path.push(trace.end_pos);
                return Ok(());
            }
        }
        Err(Reject::Miss)
    }

    /// Spans of a `len` point path traced at once: all of it for straight flights, a few ticks
    /// for arcs. Split where teammates start being hit, since the mask changes there.
    fn segments(&self, len: usize) -> Vec<(usize, usize)> {
        if len < 2 {
            return Vec::new();
        }
        let last = len - 1;
        let stride = if self.info.gravity == 0.0 {
            last
        } else {
            TRACE_STRIDE
        };
        let mut cuts: Vec<usize> = (0..last).step_by(stride).chain([last]).collect();
        let teammates = self.teammates_tick();
        if teammates > 0
            && teammates < last
            && let Err(i) = cuts.binary_search(&teammates)
        {
            cuts.insert(i, teammates);
        }
        cuts.windows(2).map(|w| (w[0], w[1])).collect()
    }

    /// The path point from which the projectile runs into teammates
    fn teammates_tick(&self) -> usize {
        let tick = Interfaces::global_vars().interval_per_tick;
        (self.info.teammates_after / tick)
            .ceil()
            .min(usize::MAX as f32) as usize
    }

    /// What the projectile collides with from path point `n` on: solid things, plus the teams
    /// whose players it runs into. CTFBaseRocket::PhysicsSolidMaskForEntity.
    fn mask(&self, n: usize) -> u32 {
        if !self.info.hits_players {
            return mask::SOLID;
        }
        let red = self.localplayer.team() == Team::Red;
        let (own, enemy) = if red {
            (mask::RED_TEAM, mask::BLUE_TEAM)
        } else {
            (mask::BLUE_TEAM, mask::RED_TEAM)
        };
        let teammates = if n >= self.teammates_tick() { own } else { 0 };
        mask::SOLID | enemy | teammates
    }

    /// The game's filter for the projectile, passing through `target` whose box is checked
    /// separately where it's predicted to be
    fn filter(&self, target: &Entity) -> TraceFilter {
        TraceFilter::simple(self.localplayer, self.info.group, Some(target))
    }

    /// The projectile's collision box
    fn hull(&self) -> (Vec3, Vec3) {
        let h = self.info.hull;
        (Vec3::new(-h, -h, -h), Vec3::new(h, h, h))
    }

    /// Cut `path` where the projectile runs into something
    pub(super) fn cut(&self, path: &mut Vec<Vec3>) {
        let (mins, maxs) = self.hull();
        let filter = TraceFilter::simple(self.localplayer, self.info.group, None);
        for (a, b) in self.segments(path.len()) {
            let trace = Interfaces::engine_trace().trace_hull(
                &path[a],
                &path[b],
                &mins,
                &maxs,
                self.mask(a),
                &filter,
            );
            if trace.did_hit() {
                // Points from a in a straight span aren't evenly spaced, keep those before the hit
                let hit = a as f32 + trace.fraction * (b - a) as f32;
                path.truncate(hit as usize + 1);
                path.push(trace.end_pos);
                return;
            }
        }
    }

    /// Whether the projectile reaches `point` without running into anything first
    pub(super) fn clear(
        &self,
        path: &[Vec3],
        target: &Entity,
        point: &Vec3,
        tolerance: f32,
    ) -> bool {
        let (mins, maxs) = self.hull();
        let filter = self.filter(target);
        for (a, b) in self.segments(path.len()) {
            count_trace();
            let trace = Interfaces::engine_trace().trace_hull(
                &path[a],
                &path[b],
                &mins,
                &maxs,
                self.mask(a),
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
