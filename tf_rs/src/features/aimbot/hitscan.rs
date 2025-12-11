use core::f32;

use crate::{
    config::Config,
    globals::{Globals, Target},
    helpers,
    interfaces::Interfaces,
    types::{ClassId, Entity, Player, UserCmd, Vec3, Weapon, user_cmd::Buttons},
};

pub fn run(
    localplayer: &Player,
    weapon: &Weapon,
    cmd: &mut UserCmd,
    globals: &mut Globals,
    config: &Config,
) {
    // TODO: if target_select_method == CLOSEST_FOV, else if target_select_method == CLOSEST_DISTANCE
    let (target, aim_angle) = closest_fov_target(localplayer, weapon, &cmd.view_angles, config);

    let should_headshot = target.as_ref().map(|t| t.should_headshot).unwrap_or(false);

    globals.target = target;

    let Some(aim_angle) = aim_angle else {
        return;
    };

    if should_headshot && weapon.spread() > 0.0 {
        return;
    }

    let use_key = config.aimbot.key.use_key;
    let wants_shot = (use_key && globals.aimbot_key_down)
        || (!use_key && (cmd.buttons & Buttons::InAttack as i32) != 0);

    if wants_shot && localplayer.can_attack() {
        cmd.view_angles = aim_angle;
        if use_key {
            cmd.buttons |= Buttons::InAttack as i32;
        }
    }
}

fn closest_fov_target(
    localplayer: &Player,
    weapon: &Weapon,
    view_angle: &Vec3,
    config: &Config,
) -> (Option<Target>, Option<Vec3>) {
    let shoot_pos = localplayer.eye_pos();
    let max_clients = Interfaces::engine_client().get_max_clients();
    let entity_list = Interfaces::entity_list();

    let mut smallest_fov = f32::MAX;
    let mut target_angle = None;
    let mut target = None;

    for i in 1..=entity_list.max_entities() {
        if i <= max_clients
            && let Some(player) = entity_list.get_client_entity::<Player>(i)
        {
            if &player == localplayer
                || player.is_dormant()
                || player.is_dead()
                || player.team() == localplayer.team()
            {
                continue;
            }

            let should_headshot = player.health() > 50 && weapon.can_headshot();

            let bone_id = if should_headshot {
                player.head_bone_id()
            } else {
                1 // Torso
            };

            let Some(target_pos) = player.get_bone_position(bone_id) else {
                continue;
            };

            let aim_angle = helpers::calculate_angle(&shoot_pos, &target_pos);
            let fov = view_angle.fov_to(&aim_angle);

            if fov < smallest_fov
                && fov <= config.aimbot.fov as f32
                && helpers::is_ent_visible(&shoot_pos, &target_pos, &player, localplayer)
            {
                smallest_fov = fov;
                target_angle = Some(aim_angle);
                target = Some(Target {
                    target_index: i,
                    should_headshot,
                    projectile_pred: None,
                });
            }
        } else if config.aimbot.building_aim
            && let Some(entity) = entity_list.get_client_entity::<Entity>(i)
        {
            if entity.is_dormant() || entity.team() == localplayer.team() {
                continue;
            }

            if !matches!(
                entity.class_id(),
                Some(ClassId::Sentry) | Some(ClassId::Dispenser) | Some(ClassId::Teleporter)
            ) {
                continue;
            }

            let target_pos = entity.origin();
            let mins = entity.mins();
            let maxs = entity.maxs();
            let target_pos = Vec3 {
                x: target_pos.x + (mins.x + maxs.x) / 2.,
                y: target_pos.y + (mins.y + maxs.y) / 2.,
                z: target_pos.z + (mins.z + maxs.z) / 2.,
            };

            let aim_angle = helpers::calculate_angle(&shoot_pos, &target_pos);
            let fov = view_angle.fov_to(&aim_angle);

            if fov < smallest_fov
                && fov <= config.aimbot.fov as f32
                && helpers::is_ent_visible(&shoot_pos, &target_pos, &entity, localplayer)
            {
                smallest_fov = fov;
                target_angle = Some(aim_angle);
                target = Some(Target {
                    target_index: i,
                    should_headshot: false,
                    projectile_pred: None,
                });
            }
        }
    }

    (target, target_angle)
}
