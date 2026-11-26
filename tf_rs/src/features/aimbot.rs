use core::f32;

use crate::{
    config::Config,
    features::{
        backtrack::{self, Choice},
        melee, projectile,
    },
    globals::{Globals, Target},
    helpers,
    interfaces::{Interfaces, mask},
    player_db,
    types::{ClassId, Entity, Player, Ray, TraceFilter, UserCmd, Vec3, Weapon, user_cmd::Buttons},
};

struct Candidate {
    target: Target,
    angle: Vec3,
    fov: f32,
    /// Simulation time of the backtrack record aimed at, none for where the target is now
    sim_time: Option<f32>,
}

fn world_visible(from: &Vec3, to: &Vec3) -> bool {
    let filter = TraceFilter::world_only();
    Interfaces::engine_trace()
        .trace(&Ray::init(from, to), mask::BULLET, &filter)
        .fraction
        >= 1.0
}

/// Aim with the active weapon, returns which backtrack tick the command should use
pub fn run(localplayer: &Player, cmd: *mut UserCmd, config: &Config) -> Choice {
    let cmd = unsafe { &mut *cmd };
    if !config.aimbot.master || Globals::read().is_spectated {
        Globals::write().target = None;
        return Choice::Auto;
    }

    let Some(weapon) = localplayer.active_weapon() else {
        Globals::write().target = None;
        return Choice::Auto;
    };

    if weapon.is_melee() && config.aimbot.melee {
        return melee::run(localplayer, &weapon, cmd, config);
    }

    if projectile::is_projectile_weapon(&weapon) {
        if config.aimbot.projectile {
            projectile::run(localplayer, &weapon, cmd, config);
        } else {
            Globals::write().target = None;
        }
        // Projectiles aren't lag compensated
        return Choice::Keep;
    }

    if !weapon.is_hitscan() {
        Globals::write().target = None;
        return Choice::Auto;
    }

    let best = get_target(localplayer, &weapon, &cmd.view_angles, config);
    Globals::write().target = best.as_ref().map(|c| c.target.clone());

    let Some(best) = best else {
        return Choice::Auto;
    };
    // Shoot at the record we aimed at, buildings and the present don't need a tick
    let choice = match best.sim_time {
        Some(sim_time) => Choice::Record(best.target.target_index, sim_time),
        None => Choice::Keep,
    };

    if best.target.should_headshot && weapon.spread() > 0.0 {
        return choice;
    }

    let use_key = config.aimbot.key.use_key;
    let wants_shot = (use_key && Globals::read().aimbot_key_down)
        || (!use_key && (cmd.buttons & Buttons::InAttack as i32) != 0);

    if wants_shot && localplayer.can_attack() {
        cmd.view_angles = best.angle;
        if use_key {
            cmd.buttons |= Buttons::InAttack as i32;
        }
    }

    choice
}

// TODO: Add sentry + other entity checks
fn get_target(
    localplayer: &Player,
    weapon: &Weapon,
    view_angle: &Vec3,
    config: &Config,
) -> Option<Candidate> {
    let shoot_pos = localplayer.eye_pos();
    let mut best: Option<Candidate> = None;
    // Priority category targets win over any other
    let mut priority: Option<Candidate> = None;

    for (i, player) in helpers::enemies(localplayer, config.aimbot.ignore_invis) {
        let player_cat = if config.aimbot.ignore_category > 0 || config.aimbot.priority_category > 0
        {
            let info = Interfaces::engine_client().get_player_info(i);
            let guid = str::from_utf8(&info.guid)
                .unwrap_or("")
                .trim_end_matches('\0');
            player_db::get(guid).index()
        } else {
            0
        };

        let is_priority =
            config.aimbot.priority_category > 0 && player_cat == config.aimbot.priority_category;

        if !is_priority
            && config.aimbot.ignore_category > 0
            && player_cat == config.aimbot.ignore_category
        {
            continue;
        }

        let headshot = player.health() > 50 && weapon.can_headshot();

        // With backtrack the server rewinds to whichever record we pick, so aim at those
        let points: Vec<(Vec3, Option<f32>)> = if config.backtrack.enabled {
            backtrack::aim_points(i, config)
                .into_iter()
                .map(|(sim_time, head, body)| (if headshot { head } else { body }, Some(sim_time)))
                .collect()
        } else {
            let bone_id = if headshot {
                player.head_bone_id()
            } else {
                1 // Torso
            };
            player
                .get_bone_position(bone_id)
                .map(|pos| (pos, None))
                .into_iter()
                .collect()
        };

        let slot = if is_priority {
            &mut priority
        } else {
            &mut best
        };
        for (pos, sim_time) in points {
            let angle = helpers::calculate_angle(&shoot_pos, &pos);
            let fov = view_angle.fov_to(&angle);
            if fov > config.aimbot.fov as f32 || slot.as_ref().is_some_and(|b| b.fov <= fov) {
                continue;
            }
            let visible = match sim_time {
                Some(_) => world_visible(&shoot_pos, &pos),
                None => helpers::is_ent_visible(&shoot_pos, &pos, &player, localplayer),
            };
            if visible {
                *slot = Some(Candidate {
                    target: Target {
                        target_index: i,
                        should_headshot: headshot,
                    },
                    angle,
                    fov,
                    sim_time,
                });
            }
        }
    }

    if !config.aimbot.building_aim {
        return priority.or(best);
    }

    let first_building = Interfaces::engine_client().get_max_clients() + 1;
    for i in first_building..=Interfaces::entity_list().max_entities() {
        let Some(entity) = Interfaces::entity_list().get_client_entity::<Entity>(i) else {
            continue;
        };
        if entity.is_dormant() || entity.team() == localplayer.team() {
            continue;
        }

        if !matches!(
            entity.class_id(),
            Some(ClassId::Sentry) | Some(ClassId::Dispenser) | Some(ClassId::Teleporter)
        ) {
            continue;
        }

        let entity_pos = entity.origin();
        let mins = entity.mins();
        let maxs = entity.maxs();
        let entity_pos = Vec3 {
            x: entity_pos.x + (mins.x + maxs.x) / 2.,
            y: entity_pos.y + (mins.y + maxs.y) / 2.,
            z: entity_pos.z + (mins.z + maxs.z) / 2.,
        };

        let angle = helpers::calculate_angle(&shoot_pos, &entity_pos);
        let fov = view_angle.fov_to(&angle);

        if fov <= config.aimbot.fov as f32
            && !best.as_ref().is_some_and(|b| b.fov <= fov)
            && helpers::is_ent_visible(&shoot_pos, &entity_pos, &entity, localplayer)
        {
            best = Some(Candidate {
                target: Target {
                    target_index: i,
                    should_headshot: false,
                },
                angle,
                fov,
                sim_time: None,
            });
        }
    }

    priority.or(best)
}
