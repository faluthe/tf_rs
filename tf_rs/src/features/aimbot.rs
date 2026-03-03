use core::f32;

use crate::{
    config::Config,
    globals::{Globals, Target},
    helpers,
    interfaces::Interfaces,
    player_db,
    types::{ClassId, Entity, Player, UserCmd, Vec3, Weapon, user_cmd::Buttons},
};

pub fn run(localplayer: &Player, cmd: *mut UserCmd, config: &Config) {
    let cmd = unsafe { &mut *cmd };
    if !config.aimbot.master {
        Globals::write().target = None;
        return;
    }

    let Some(weapon) = localplayer.active_weapon() else {
        Globals::write().target = None;
        return;
    };

    if !weapon.is_hitscan() {
        Globals::write().target = None;
        return;
    }

    let (target, aim_angle) = get_target(localplayer, &weapon, &cmd.view_angles, config);
    let should_headshot = target.as_ref().map(|t| t.should_headshot).unwrap_or(false);

    Globals::write().target = target;

    let Some(aim_angle) = aim_angle else {
        return;
    };

    if should_headshot && weapon.spread() > 0.0 {
        return;
    }

    let use_key = config.aimbot.key.use_key;
    let wants_shot = (use_key && Globals::read().aimbot_key_down)
        || (!use_key && (cmd.buttons & Buttons::InAttack as i32) != 0);

    if wants_shot && localplayer.can_attack() {
        cmd.view_angles = aim_angle;
        if use_key {
            cmd.buttons |= Buttons::InAttack as i32;
        }
    }
}

// TODO: Add sentry + other entity checks
fn get_target(
    localplayer: &Player,
    weapon: &Weapon,
    view_angle: &Vec3,
    config: &Config,
) -> (Option<Target>, Option<Vec3>) {
    let shoot_pos = localplayer.eye_pos();
    let mut smallest_fov = f32::MAX;
    let mut target_angle = None;
    let mut target = None;

    // Separate tracking for priority category targets
    let mut priority_smallest_fov = f32::MAX;
    let mut priority_target_angle = None;
    let mut priority_target = None;

    for i in 1..=Interfaces::entity_list().max_entities() {
        if i <= Interfaces::engine_client().get_max_clients()
            && let Some(player) = Interfaces::entity_list().get_client_entity::<Player>(i)
        {
            if &player == localplayer
                || player.is_dormant()
                || player.is_dead()
                || player.team() == localplayer.team()
            {
                continue;
            }

            let player_cat =
                if config.aimbot.ignore_category > 0 || config.aimbot.priority_category > 0 {
                    let info = Interfaces::engine_client().get_player_info(i);
                    let guid = str::from_utf8(&info.guid)
                        .unwrap_or("")
                        .trim_end_matches('\0');
                    player_db::get(guid).index()
                } else {
                    0
                };

            let is_priority = config.aimbot.priority_category > 0
                && player_cat == config.aimbot.priority_category;

            if !is_priority
                && config.aimbot.ignore_category > 0
                && player_cat == config.aimbot.ignore_category
            {
                continue;
            }

            let headshot = player.health() > 50 && weapon.can_headshot();

            let bone_id = if headshot {
                player.head_bone_id()
            } else {
                1 // Torso
            };
            let Some(player_pos) = player.get_bone_position(bone_id) else {
                continue;
            };

            let aim_angle = helpers::calculate_angle(&shoot_pos, &player_pos);
            let fov = view_angle.fov_to(&aim_angle);

            if fov <= config.aimbot.fov as f32
                && helpers::is_ent_visible(&shoot_pos, &player_pos, &player, localplayer)
            {
                if is_priority {
                    if fov < priority_smallest_fov {
                        priority_smallest_fov = fov;
                        priority_target_angle = Some(aim_angle);
                        priority_target = Some(Target {
                            target_index: i,
                            should_headshot: headshot,
                        });
                    }
                } else if fov < smallest_fov {
                    smallest_fov = fov;
                    target_angle = Some(aim_angle);
                    target = Some(Target {
                        target_index: i,
                        should_headshot: headshot,
                    });
                }
            }
        } else if let Some(entity) = Interfaces::entity_list().get_client_entity::<Entity>(i) {
            if !config.aimbot.building_aim
                || entity.is_dormant()
                || entity.team() == localplayer.team()
            {
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

            let aim_angle = helpers::calculate_angle(&shoot_pos, &entity_pos);
            let fov = view_angle.fov_to(&aim_angle);

            if fov < smallest_fov
                && fov <= config.aimbot.fov as f32
                && helpers::is_ent_visible(&shoot_pos, &entity_pos, &entity, localplayer)
            {
                smallest_fov = fov;
                target_angle = Some(aim_angle);
                target = Some(Target {
                    target_index: i,
                    should_headshot: false,
                });
            }
        }
    }

    if priority_target.is_some() {
        (priority_target, priority_target_angle)
    } else {
        (target, target_angle)
    }
}
