//! Projectile aimbot.
//!
//! Targets are simulated forward tick by tick, and the first tick a projectile can reach them in
//! time is aimed at. Explosives that can't reach a target directly aim at a nearby surface the
//! blast would reach them from.

mod arcs;
mod points;
mod solver;
mod tracking;
mod weapon;

use std::{cmp::Ordering, sync::atomic};

use crate::{
    config::Config,
    features::{
        aimbot,
        movesim::{MoveSim, Strafe},
        shot_log,
    },
    globals::{Globals, PredictionPaths, Target},
    helpers,
    interfaces::Interfaces,
    types::{Entity, Hull, Player, UserCmd, Vec3, Weapon, user_cmd::Buttons},
};
use points::{aim_points, splash_points};
use solver::{Solution, Solve, Solver};
use tracking::{landings, track_in_flight};
use weapon::{ProjectileInfo, charge_time, weapon_info};

pub use tracking::{forget_projectiles, our_projectiles};
pub use weapon::is_projectile_weapon;

const MAX_TARGETS: usize = 3;
/// Seconds of flight drawn along the view, enough to cross any map
const VIEW_PATH_TIME: f32 = 3.0;
/// Ticks per target splash is looked for, from the first one the projectile could make it by
const SPLASH_TICKS: i32 = 12;

/// Traces run by the current search, for the perf log
static TRACES: atomic::AtomicU32 = atomic::AtomicU32::new(0);

fn count_trace() {
    TRACES.fetch_add(1, atomic::Ordering::Relaxed);
}

struct Found {
    index: i32,
    building: bool,
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
    /// The projectile flies past the target, or gets to it before the target does
    Miss,
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

    // Charged weapons fire on release: with the key they charge while it's held and release on a
    // shot, otherwise the shot is aimed as the user releases. Others aim even while the weapon
    // isn't ready, since attack cuts a reload short and fires as soon as it can.
    let looking = if info.charge {
        if key && key_down {
            cmd.buttons |= ATTACK;
        }
        charging && aimbot::wants_aim(aim, key && key_down, !attacking)
    } else {
        aimbot::wants_aim(aim, key_down, attacking)
    };
    if !looking {
        Globals::write().target = None;
        // Still look for a shot to draw, without aiming or firing it
        if aim.draw_prediction && aim.draw_aim_point {
            let (found, _) = find_target(localplayer, cmd, config, info);
            if let Some(found) = found {
                preview(&found.paths);
            }
        }
        return;
    }

    let logging = aim.log_prediction;
    let started = std::time::Instant::now();
    TRACES.store(0, atomic::Ordering::Relaxed);
    let (found, diagnostics) = find_target(localplayer, cmd, config, info);
    if logging {
        shot_log::search_time(
            started.elapsed().as_secs_f32() * 1000.0,
            TRACES.load(atomic::Ordering::Relaxed),
        );
    }
    Globals::write().target = found.as_ref().map(|f| Target {
        target_index: f.index,
        should_headshot: info.headshot && !f.building,
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
        let land_time = s.start_time + (s.choke + s.latency_ticks + s.ticks) as f32 * tick;
        tracking::record(found.index, land_time, found.time);
    }
    // The shot log follows players
    if logging && fires && !found.building {
        shot_log::shot(
            localplayer,
            &found.shot,
            &found.paths.projectile,
            found.time,
            found.fov,
        );
    }

    if aim.draw_prediction && aim.draw_aim_point {
        preview(&found.paths);
    }
    // Only what's fired gets its paths drawn, a line for a shot that never left looks like a miss
    if aim.draw_prediction && fires {
        let now = Interfaces::global_vars().realtime;
        let mut paths = found.paths;
        paths.until = now + found.time + helpers::round_trip();
        Globals::write().prediction = Some(paths);
    }
}

/// Seconds a shot that isn't fired stays drawn, long enough to bridge the ticks between searches
const PREVIEW_TIME: f32 = 0.1;

/// Mark where the shot in `paths` would land, without firing it
fn preview(paths: &PredictionPaths) {
    if let Some(end) = paths.projectile.last() {
        let until = Interfaces::global_vars().realtime + PREVIEW_TIME;
        Globals::write().aim_preview = Some((*end, until));
    }
}

/// Where a projectile fired from the held weapon along the view would go, until it hits something.
/// Taken from the frame's camera rather than the last command, so it follows the view smoothly.
pub fn view_path(localplayer: &Player) -> Option<Vec<Vec3>> {
    let weapon = localplayer.active_weapon()?;
    if !is_projectile_weapon(&weapon) {
        return None;
    }
    let info = weapon_info(localplayer, &weapon)?;
    let (camera, angles) = helpers::view::camera()?;
    // The camera is the eye in firstperson and moves with it between ticks, unlike our origin
    let eye = localplayer.eye_pos();
    let eye = if camera.distance_to(&eye) < 16.0 {
        camera
    } else {
        eye
    };
    let solver = Solver::for_path(localplayer, eye, info);
    let mut path = solver.path(&angles, info.lifetime.min(VIEW_PATH_TIME))?;
    solver.cut(&mut path);
    Some(path)
}

fn find_target(
    localplayer: &Player,
    cmd: &UserCmd,
    config: &Config,
    info: ProjectileInfo,
) -> (Option<Found>, Vec<shot_log::TargetDiagnostics>) {
    let aim = &config.aimbot;
    let tick = Interfaces::global_vars().interval_per_tick;
    let eye = localplayer.eye_pos();
    let max_ticks = ((aim.max_sim_time as f32 / 1000.0).min(info.lifetime) / tick) as i32;
    let solver = Solver::new(localplayer, eye, info, max_ticks as f32 * tick);

    // Closest to the crosshair first, simulating everyone would be too slow
    let mut targets: Vec<(f32, Player)> = helpers::enemies(localplayer, aim.ignore_invis)
        .map(|(_, p)| {
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

    let landings = if aim.wait_for_shot {
        landings()
    } else {
        Vec::new()
    };
    let search = Search {
        cmd,
        strafe: Strafe::from(aim),
        tick,
        // The server fires on this command at our tick base, this far past the newest tick we've
        // been sent. Targets' simulation times can't be used, each runs ahead by its own amount.
        latency_ticks: (localplayer.tick_base() - Interfaces::global_vars().tickcount).max(0),
        max_ticks,
        use_splash: aim.splash && info.splash_radius > 0.0,
        splash_slack: info.splash_radius / solver.speed,
        solver,
    };

    let mut best: Option<Found> = None;
    let mut diagnostics = Vec::new();
    for (center_fov, target) in targets {
        let mut diag = shot_log::TargetDiagnostics {
            index: target.index(),
            fov: center_fov,
            distance: target.origin().distance_to(&eye),
            on_ground: target.is_on_ground(),
            ..Default::default()
        };
        let wait_for = landings
            .iter()
            .filter(|(t, _)| *t == target.index())
            .map(|(_, land_time)| *land_time)
            .min_by(f32::total_cmp);
        if let Some(found) = search.target(&target, wait_for, &mut diag)
            && best.as_ref().is_none_or(|b| found.fov < b.fov)
        {
            best = Some(found);
        }
        diagnostics.push(diag);
    }

    let buildings = helpers::enemy_buildings(localplayer).filter(|_| aim.building_aim);
    for (index, building) in buildings {
        let center = building.origin() + (building.mins() + building.maxs()) * 0.5;
        let center_fov = cmd
            .view_angles
            .fov_to(&helpers::calculate_angle(&eye, &center));
        if center_fov > aim.fov as f32 {
            continue;
        }
        let mut diag = shot_log::TargetDiagnostics {
            index,
            fov: center_fov,
            distance: center.distance_to(&eye),
            on_ground: true,
            ..Default::default()
        };
        let wait_for = landings
            .iter()
            .filter(|(t, _)| *t == index)
            .map(|(_, land_time)| *land_time)
            .min_by(f32::total_cmp);
        if let Some(found) = search.building(&building, index, wait_for, &mut diag)
            && best.as_ref().is_none_or(|b| found.fov < b.fov)
        {
            best = Some(found);
        }
        diagnostics.push(diag);
    }
    (best, diagnostics)
}

/// What the search of each target shares
struct Search<'a> {
    solver: Solver<'a>,
    cmd: &'a UserCmd,
    strafe: Strafe,
    tick: f32,
    /// Ticks targets move before the server fires the projectile
    latency_ticks: i32,
    max_ticks: i32,
    use_splash: bool,
    /// How much sooner than the target a shot at a spot near them can land
    splash_slack: f32,
}

impl Search<'_> {
    /// A shot at the first tick `target` can be hit on, with why other shots were turned down
    /// in `diag`. Shots landing at or after `wait_for` aren't taken.
    fn target(
        &self,
        target: &Player,
        wait_for: Option<f32>,
        diag: &mut shot_log::TargetDiagnostics,
    ) -> Option<Found> {
        let (info, eye, tick) = (&self.solver.info, self.solver.eye, self.tick);
        // Where the head is from their origin, read before the sim moves them
        let head = info
            .headshot
            .then(|| target.head_center())
            .flatten()
            .map(|head| head - target.origin());
        let Some(mut sim) = MoveSim::new(target, None, self.strafe) else {
            diag.no_sim = true;
            return None;
        };
        let start_time = target.simulation_time();
        let choke = sim.choke();
        // Targets keep moving until the projectile leaves, the sim has already caught up to now
        let latency_ticks = self.latency_ticks;
        for _ in 0..latency_ticks {
            sim.run_tick();
        }

        let direct_tolerance = info.hull.max(4.0) + 8.0;
        let splash_tolerance = info.hull.max(4.0) + 12.0;
        let mut splash_ticks = 0;
        for ticks in 1..=self.max_ticks {
            let land_time = start_time + (choke + latency_ticks + ticks) as f32 * tick;
            if wait_for.is_some_and(|w| land_time >= w) {
                diag.waiting = true;
                return None;
            }
            sim.run_tick();
            let origin = sim.predicted_origin();
            let target_box = Hull {
                origin,
                mins: target.mins(),
                maxs: target.maxs(),
            };
            let mut hit = aim_points(target, info, sim.on_ground(), head)
                .enumerate()
                .find_map(|(n, offset)| {
                    let extra = if n == 0 && info.splash { 8.0 } else { 0.0 };
                    let point = origin + offset;
                    self.attempt(
                        target,
                        &point,
                        ticks,
                        direct_tolerance + extra,
                        Some(&target_box),
                        diag,
                    )
                    .map(|hit| (hit, point, false))
                });

            // Splash spots are at most the blast radius closer, so only look once the projectile
            // could nearly reach the target
            if hit.is_none() && self.use_splash && splash_ticks < SPLASH_TICKS {
                let center = origin + Vec3::new(0.0, 0.0, target.maxs().z / 2.0);
                let in_window = self
                    .solver
                    .trajectory(&eye, &center)
                    .is_some_and(|(_, time)| time - self.splash_slack <= ticks as f32 * tick);
                if in_window {
                    splash_ticks += 1;
                    hit = splash_points(self.solver.localplayer, origin, target, info, &eye)
                        .iter()
                        .find_map(|point| {
                            self.attempt(target, point, ticks, splash_tolerance, None, diag)
                                .map(|hit| (hit, *point, true))
                        });
                }
            }

            let Some(((solution, path, fov), point, splash)) = hit else {
                continue;
            };
            let mut player_path = sim.path.clone();
            player_path.push(sim.origin());
            return Some(Found {
                index: target.index(),
                building: false,
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
                    launch: self.solver.launch(&solution.angles).unwrap_or_default(),
                    gravity: info.gravity,
                    drag: info.drag,
                },
            });
        }
        None
    }

    /// A shot at `building`, which stays where it is. Shots landing at or after `wait_for`
    /// aren't taken.
    fn building(
        &self,
        building: &Entity,
        index: i32,
        wait_for: Option<f32>,
        diag: &mut shot_log::TargetDiagnostics,
    ) -> Option<Found> {
        let (info, tick) = (&self.solver.info, self.tick);
        let (origin, mins, maxs) = (building.origin(), building.mins(), building.maxs());
        let target_box = Hull { origin, mins, maxs };
        let middle = origin + (mins + maxs) * 0.5;
        let base = Vec3::new(middle.x, middle.y, origin.z + mins.z + 5.0);
        // Explosives at its base blast it as well, and the floor is harder to miss
        let points = if info.splash {
            [base, middle]
        } else {
            [middle, base]
        };
        // Nothing to simulate, the newest tick is where it is
        let start_time = Interfaces::global_vars().tickcount as f32 * tick;
        let latency_ticks = self.latency_ticks;
        let tolerance = info.hull.max(4.0) + 8.0;

        let direct = points.iter().find_map(|point| {
            // It doesn't move, so the shot lands whenever the projectile gets there
            let ticks = match self.solver.solve(point, self.max_ticks) {
                Solve::Good(solution) => (solution.time / tick) as i32 + 1,
                Solve::Late => {
                    diag.count(Reject::Late);
                    return None;
                }
                Solve::Bad => {
                    diag.count(Reject::Bad);
                    return None;
                }
            };
            self.attempt(building, point, ticks, tolerance, Some(&target_box), diag)
                .map(|hit| (hit, *point, ticks, false))
        });
        let hit = direct.or_else(|| {
            if !self.use_splash {
                return None;
            }
            let eye = self.solver.eye;
            let localplayer = self.solver.localplayer;
            splash_points(localplayer, origin, building, info, &eye)
                .iter()
                .find_map(|point| {
                    let Solve::Good(solution) = self.solver.solve(point, self.max_ticks) else {
                        return None;
                    };
                    let ticks = (solution.time / tick) as i32 + 1;
                    self.attempt(building, point, ticks, tolerance + 4.0, None, diag)
                        .map(|hit| (hit, *point, ticks, true))
                })
        });
        let ((solution, path, fov), point, ticks, splash) = hit?;

        let land_time = start_time + (latency_ticks + ticks) as f32 * tick;
        if wait_for.is_some_and(|w| land_time >= w) {
            diag.waiting = true;
            return None;
        }
        Some(Found {
            index,
            building: true,
            angles: solution.angles,
            time: solution.time,
            fov,
            paths: PredictionPaths {
                player: Vec::new(),
                projectile: path,
                until: 0.0,
            },
            shot: shot_log::Prediction {
                target: index,
                start_time,
                choke: 0,
                latency_ticks,
                ticks,
                origin,
                point,
                splash,
                target_on_ground: true,
                launch: self.solver.launch(&solution.angles).unwrap_or_default(),
                gravity: info.gravity,
                drag: info.drag,
            },
        })
    }

    /// An unobstructed shot landing on `point` within `ticks`, counting why not in `diag`. With
    /// `predicted` it has to hit that box, otherwise only reach the surface at `point`.
    fn attempt(
        &self,
        target: &Entity,
        point: &Vec3,
        ticks: i32,
        tolerance: f32,
        predicted: Option<&Hull>,
        diag: &mut shot_log::TargetDiagnostics,
    ) -> Option<(Solution, Vec<Vec3>, f32)> {
        let solver = &self.solver;
        let reject = match solver.solve(point, ticks) {
            Solve::Good(solution) => {
                let fov = self.cmd.view_angles.fov_to(&solution.angles);
                let tick = Interfaces::global_vars().interval_per_tick;
                let shot = match predicted {
                    Some(predicted) => solver
                        .path(&solution.angles, ticks as f32 * tick)
                        .ok_or(Reject::Bad)
                        .and_then(|mut path| {
                            solver
                                .hits(&mut path, target, predicted, point, tolerance, ticks)
                                .map(|()| path)
                        }),
                    None => match solver.path(&solution.angles, solution.time) {
                        None => Err(Reject::Bad),
                        Some(path) if solver.clear(&path, target, point, tolerance) => Ok(path),
                        Some(_) => Err(Reject::Blocked),
                    },
                };
                match shot {
                    Ok(path) => return Some((solution, path, fov)),
                    Err(reject) => reject,
                }
            }
            Solve::Late => Reject::Late,
            Solve::Bad => Reject::Bad,
        };
        diag.count(reject);
        None
    }
}
