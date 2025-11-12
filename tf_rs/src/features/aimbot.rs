use core::f32;

use crate::{
    cfg_enabled, helpers,
    interfaces::Interfaces,
    types::{Player, UserCmd, Vec3, user_cmd::Buttons},
};

pub fn run(localplayer: &Player, cmd: *mut UserCmd) {
    let cmd = unsafe { &mut *cmd };
    if !cfg_enabled!(aimbot) {
        return;
    }

    let (_, Some(aim_angle)) = get_target(localplayer, &cmd.view_angles) else {
        return;
    };

    let wants_shot = (cmd.buttons & Buttons::InAttack as i32) != 0;

    if wants_shot && localplayer.can_attack() {
        cmd.view_angles = aim_angle;
    }
}

// TODO: Add sentry + other entity checks
pub fn get_target(localplayer: &Player, view_angle: &Vec3) -> (Option<Player>, Option<Vec3>) {
    let shoot_pos = localplayer.eye_pos();
    let mut smallest_fov = f32::MAX;
    let mut target = None;
    let mut target_angle = None;

    for i in 1..=Interfaces::engine_client().get_max_clients() {
        if let Some(player) = Interfaces::entity_list().get_client_entity::<Player>(i) {
            if &player == localplayer
                || player.is_dormant()
                || player.is_dead()
                || player.team() == localplayer.team()
            {
                continue;
            }

            let bone_id = if player.health() < 50 || !localplayer.can_headshot() {
                1 // Torso
            } else {
                player.head_bone_id()
            };
            let Some(player_pos) = player.get_bone_position(bone_id) else {
                continue;
            };

            if !helpers::is_ent_visible(&shoot_pos, &player_pos, &player, localplayer) {
                continue;
            }

            let aim_angle = helpers::calculate_angle(&shoot_pos, &player_pos);
            let fov = view_angle.fov_to(&aim_angle);

            if fov < smallest_fov {
                smallest_fov = fov;
                target = Some(player);
                target_angle = Some(aim_angle);
            }
        }
    }

    (target, target_angle)
}
