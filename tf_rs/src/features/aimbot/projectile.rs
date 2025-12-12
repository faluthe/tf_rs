use crate::{
    config::Config,
    globals::{Globals, ProjectileTarget, Target},
    helpers,
    interfaces::Interfaces,
    types::{Entity, Player, UserCmd, Vec3, Weapon, user_cmd::Buttons, weapon::WeaponClass},
};

pub fn run(
    localplayer: &Player,
    weapon: &Weapon,
    cmd: &mut UserCmd,
    globals: &mut Globals,
    config: &Config,
) {
    let shoot_pos = projectile_fire_setup(
        &cmd.view_angles,
        &localplayer.eye_pos(),
        &weapon.projectile_fire_offset(),
    );
    // TODO: if target_select_method == CLOSEST_FOV, else if target_select_method == CLOSEST_DISTANCE
    let (target, aim_angle) =
        closest_fov_target_pred(localplayer, weapon, &cmd.view_angles, &shoot_pos, config);

    globals.target = target;

    let Some(aim_angle) = aim_angle else {
        return;
    };

    let use_key = config.aimbot.key.use_key;
    let wants_shot = (use_key && globals.aimbot_key_down)
        || (!use_key && (cmd.buttons & Buttons::InAttack as i32) != 0);

    if wants_shot {
        cmd.view_angles = aim_angle;
        if use_key {
            cmd.buttons |= Buttons::InAttack as i32;
        }
    }
}

fn ballistic_time_and_pitch(
    weapon: &Weapon,
    Vec3 { x: dx, y: dy, z: h }: Vec3,
) -> Option<(f32, Option<Vec3>)> {
    let v = weapon.projectile_speed()?;
    let g = 800.0 * weapon.projectile_gravity()?;
    let h_dist = (dx * dx + dy * dy).sqrt();

    let v2 = v * v;
    let r2 = h_dist * h_dist;

    let a = g * r2 / (2.0 * v2);
    let disc = r2 - 4.0 * a * (a + h);
    if disc < 0.0 {
        return None;
    }

    let sqrt_disc = disc.sqrt();
    let u1 = (h_dist - sqrt_disc) / (2.0 * a);
    let u2 = (h_dist + sqrt_disc) / (2.0 * a);
    let u = if u1.abs() < u2.abs() { u1 } else { u2 };

    let cos_theta = 1.0 / (1.0 + u * u).sqrt();
    let t = h_dist / (v * cos_theta);
    let theta_deg = u.atan().to_degrees();

    let aim_angles = Vec3 {
        x: -theta_deg,
        y: dy.atan2(dx).to_degrees(),
        z: 0.0,
    };

    Some((t, Some(aim_angles)))
}

fn closest_fov_target_pred(
    localplayer: &Player,
    weapon: &Weapon,
    view_angle: &Vec3,
    shoot_pos: &Vec3,
    config: &Config,
) -> (Option<Target>, Option<Vec3>) {
    let Some(projectile_speed) = weapon.projectile_speed() else {
        return (None, None);
    };

    let max_clients = Interfaces::engine_client().get_max_clients();
    let entity_list = Interfaces::entity_list();
    let step_time = config.aimbot.projectile.step_time;
    let tolerance = config.aimbot.projectile.tolerance;

    let mut smallest_fov = f32::MAX;
    let mut target_angles = None;
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

            let cur_pos = if matches!(weapon.weapon_class(), WeaponClass::CompoundBow) {
                player.get_bone_position(player.head_bone_id())
            } else {
                Some(player.origin())
            };

            let Some(cur_pos) = cur_pos else {
                continue;
            };

            let is_in_air = !player.is_on_ground();
            let velocity = player.velocity();

            for step in 0..config.aimbot.projectile.max_steps {
                let t = step as f32 * step_time;
                let pred_pos = if is_in_air {
                    Vec3 {
                        x: cur_pos.x + velocity.x * t,
                        y: cur_pos.y + velocity.y * t,
                        z: cur_pos.z + velocity.z * t - 0.5 * 800.0 * t * t,
                    }
                } else {
                    Vec3 {
                        x: cur_pos.x + velocity.x * t,
                        y: cur_pos.y + velocity.y * t,
                        z: cur_pos.z + velocity.z * t,
                    }
                };

                let distance = shoot_pos.distance_to(&pred_pos);

                let should_headshot = weapon.can_headshot();
                let solution = if weapon.uses_gravity() {
                    ballistic_time_and_pitch(weapon, pred_pos - *shoot_pos)
                } else {
                    Some((distance / projectile_speed, None))
                };

                let Some((travel_time, aim_angles)) = solution else {
                    continue;
                };

                if (travel_time - t).abs() > tolerance {
                    continue;
                }

                let aim_angles = aim_angles
                    .unwrap_or(helpers::calculate_angle(&localplayer.eye_pos(), &pred_pos));

                let fov = view_angle.fov_to(&aim_angles);

                if fov < smallest_fov
                    && fov <= config.aimbot.fov as f32
                    && helpers::is_pos_visible(shoot_pos, &pred_pos, localplayer)
                {
                    smallest_fov = fov;
                    target_angles = Some(aim_angles);
                    target = Some(Target {
                        target_index: i,
                        should_headshot,
                        projectile_pred: Some(ProjectileTarget {
                            proj_start: shoot_pos.clone(),
                            proj_end: pred_pos,
                            travel_time,
                            direction: angles_to_direction_vector(&aim_angles).0,
                        }),
                    })
                }
            }
        } else if config.aimbot.building_aim
            && let Some(_) = entity_list.get_client_entity::<Entity>(i)
        {
            // TODO
        }
    }

    (target, target_angles)
}

fn projectile_fire_setup(view_angles: &Vec3, shoot_pos: &Vec3, offset: &Vec3) -> Vec3 {
    let (forward, right, up) = angles_to_direction_vector(view_angles);

    Vec3 {
        x: shoot_pos.x + (forward.x * offset.x) + (right.x * offset.y) + (up.x * offset.z),
        y: shoot_pos.y + (forward.y * offset.x) + (right.y * offset.y) + (up.y * offset.z),
        z: shoot_pos.z + (forward.z * offset.x) + (right.z * offset.y) + (up.z * offset.z),
    }
}

fn angles_to_direction_vector(angles: &Vec3) -> (Vec3, Vec3, Vec3) {
    let (sp, cp) = angles.x.to_radians().sin_cos(); // pitch
    let (sy, cy) = angles.y.to_radians().sin_cos(); // yaw
    let (sr, cr) = angles.z.to_radians().sin_cos(); // roll

    let forward = Vec3 {
        x: cp * cy,
        y: cp * sy,
        z: -sp,
    };

    let right = Vec3 {
        x: (-sr * sp * cy + -cr * -sy),
        y: (-sr * sp * sy + -cr * cy),
        z: -sr * cp,
    };

    let up = Vec3 {
        x: (cr * sp * cy + -sr * -sy),
        y: (cr * sp * sy + -sr * cy),
        z: cr * cp,
    };

    (forward, right, up)
}
