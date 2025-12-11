use std::sync::OnceLock;

use crate::{
    config::{Config, EntityESPConfig},
    globals::{Globals, ProjectileTarget},
    helpers,
    interfaces::{Interfaces, Surface},
    types::{BBox, ClassId, Cond, Entity, Player, RGBA, Vec3, Weapon, rgba},
};

static ESP_FONT: OnceLock<u64> = OnceLock::new();

pub fn esp_font(surface: &Surface) -> u64 {
    *ESP_FONT.get_or_init(|| {
        let font = surface.create_font();
        surface.set_font_glyph_set(font, "DejaVu Sans Mono", 14, 400, 0, 0, 0x0);
        font
    })
}

fn draw_projectile_pred(
    weapon: Option<Weapon>,
    target: &ProjectileTarget,
    should_headshot: bool,
    entity: &Entity,
    surface: &Surface,
) {
    let debug_overlay = Interfaces::debug_overlay();

    // Draw projectile prediction
    if let Some(weapon) = weapon {
        if weapon.uses_gravity() {
            let Some(velocity) = weapon.projectile_speed() else {
                return;
            };

            let Some(g) = weapon.projectile_gravity() else {
                return;
            };

            let Some(cur_pos_2d) = debug_overlay.screen_position(&target.proj_start) else {
                return;
            };

            let step_time = 0.01;
            let max_steps = (target.travel_time / step_time).ceil() as i32;
            let mut prev_x = cur_pos_2d.x as i32;
            let mut prev_y = cur_pos_2d.y as i32;

            surface.draw_set_color(255, 255, 255, 255);

            for step in 0..max_steps {
                let t = step as f32 * step_time;
                let pos = Vec3 {
                    x: target.proj_start.x + target.direction.x * velocity * t,
                    y: target.proj_start.y + target.direction.y * velocity * t,
                    z: target.proj_start.z + target.direction.z * velocity * t
                        - 0.5 * g * 800.0 * t * t,
                };

                let Some(pos_2d) = debug_overlay.screen_position(&pos) else {
                    continue;
                };

                let x = pos_2d.x as i32;
                let y = pos_2d.y as i32;

                surface.draw_line(prev_x, prev_y, x, y);

                prev_x = x;
                prev_y = y;
            }
        } else {
            let Some(start_2d) = debug_overlay.screen_position(&target.proj_start) else {
                return;
            };

            let Some(end_2d) = debug_overlay.screen_position(&target.proj_end) else {
                return;
            };

            surface.draw_set_color(255, 255, 255, 255);
            surface.draw_line(
                start_2d.x as i32,
                start_2d.y as i32,
                end_2d.x as i32,
                end_2d.y as i32,
            );
        }
    }

    // Draw entity prediction
    if !entity.is_player() {
        return;
    }

    let player = Player { ent: *entity };
    let cur_pos = if should_headshot {
        player.get_bone_position(player.head_bone_id())
    } else {
        Some(player.origin())
    };

    let Some(cur_pos) = cur_pos else {
        return;
    };

    let Some(cur_pos_2d) = debug_overlay.screen_position(&cur_pos) else {
        return;
    };

    let is_in_air = !player.is_on_ground();
    let velocity = player.velocity();
    let step_time = 0.01;
    let max_steps = (target.travel_time / step_time).ceil() as i32;
    let mut prev_x1 = cur_pos_2d.x as i32;
    let mut prev_y1 = cur_pos_2d.y as i32;

    surface.draw_set_color(255, 255, 255, 255);

    for step in 0..max_steps {
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

        let Some(pred_2d) = debug_overlay.screen_position(&pred_pos) else {
            continue;
        };

        let x1 = pred_2d.x as i32;
        let y1 = pred_2d.y as i32;

        surface.draw_line(prev_x1, prev_y1, x1, y1);

        prev_x1 = x1;
        prev_y1 = y1;
    }
}

pub fn run(localplayer: &Player, surface: &Surface, config: &Config) {
    if !config.esp.master {
        return;
    }

    let globals = Globals::read();
    let target = globals.target.as_ref();

    for i in 1..Interfaces::entity_list().max_entities() {
        if let Some(entity) = Interfaces::entity_list().get_client_entity::<Entity>(i) {
            if entity.is_dormant() || entity == localplayer.ent {
                continue;
            }

            let mut conds: Vec<(String, &RGBA)> = Vec::new();

            let is_target = if config.esp.aimbot_target && !localplayer.is_dead() {
                if let Some(target) = target {
                    if i == target.target_index {
                        conds.push(("TARGET".to_string(), &rgba::WHITE));

                        if target.should_headshot {
                            conds.push(("HS".to_string(), &rgba::RED));
                        }

                        if let Some(proj_target) = &target.projectile_pred {
                            draw_projectile_pred(
                                localplayer.active_weapon(),
                                proj_target,
                                target.should_headshot,
                                &entity,
                                surface,
                            );
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            let class_id = match entity.class_id() {
                Some(class_id) => class_id,
                None => continue,
            };

            let bbox = match class_id {
                ClassId::Player => {
                    let player = Player { ent: entity };

                    if player.is_dead() {
                        continue;
                    }

                    let friendly = player.team() == localplayer.team();

                    if friendly && !config.esp.player_friendly.bool() {
                        continue;
                    }

                    let Some(bbox) = helpers::get_bounding_box(&player) else {
                        continue;
                    };

                    let cfg = if friendly {
                        &config.esp.player_friendly
                    } else {
                        &config.esp.player_enemy
                    };

                    if cfg.boxes {
                        draw_box(
                            &bbox,
                            if is_target {
                                &rgba::ORANGE
                            } else {
                                player.team().as_rgba()
                            },
                            surface,
                        );
                    }

                    if cfg.names {
                        let name = Interfaces::engine_client().get_player_info(i).name;
                        let name = str::from_utf8(&name).unwrap_or("");

                        draw_name(
                            &bbox,
                            name,
                            if is_target {
                                &rgba::ORANGE
                            } else {
                                player.team().as_rgba()
                            },
                            surface,
                        );
                    }

                    if cfg.health {
                        draw_health(&bbox, &player, surface, false);
                    }

                    if cfg.conds {
                        if player.in_cond(Cond::Disguised) {
                            conds.push(("DISGUISED".to_string(), &rgba::WHITE));
                        }

                        if player.in_cond(Cond::Taunting) {
                            conds.push(("TAUNTING".to_string(), &rgba::WHITE));
                        }

                        if player.in_cond(Cond::Zoomed) {
                            conds.push(("ZOOMED".to_string(), &rgba::WHITE));
                        }

                        if player.is_invisible() {
                            conds.push(("INVISIBLE".to_string(), &rgba::WHITE));
                        }

                        if player.in_cond(Cond::MadMilk) {
                            conds.push(("MILKED".to_string(), &rgba::WHITE));
                        }
                    }

                    Some(bbox)
                }
                ClassId::Sentry => {
                    let friendly = entity.team() == localplayer.team();

                    if !friendly || config.esp.building_friendly.bool() {
                        building_esp(
                            &entity,
                            "Sentry Gun",
                            if friendly {
                                &config.esp.building_friendly
                            } else {
                                &config.esp.building_enemy
                            },
                            surface,
                            if is_target {
                                &rgba::ORANGE
                            } else {
                                entity.team().as_rgba()
                            },
                        )
                    } else {
                        continue;
                    }
                }
                ClassId::Dispenser => {
                    let friendly = entity.team() == localplayer.team();

                    if !friendly || config.esp.building_friendly.bool() {
                        building_esp(
                            &entity,
                            "Dispenser",
                            if friendly {
                                &config.esp.building_friendly
                            } else {
                                &config.esp.building_enemy
                            },
                            surface,
                            if is_target {
                                &rgba::ORANGE
                            } else {
                                entity.team().as_rgba()
                            },
                        )
                    } else {
                        continue;
                    }
                }
                ClassId::Teleporter => {
                    let friendly = entity.team() == localplayer.team();

                    if !friendly || config.esp.building_friendly.bool() {
                        building_esp(
                            &entity,
                            "Teleporter",
                            if friendly {
                                &config.esp.building_friendly
                            } else {
                                &config.esp.building_enemy
                            },
                            surface,
                            if is_target {
                                &rgba::ORANGE
                            } else {
                                entity.team().as_rgba()
                            },
                        )
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            let Some(bbox) = bbox else {
                continue;
            };

            for (j, (cond, color)) in conds.iter().enumerate() {
                surface.draw_set_text_color(color.r, color.g, color.b, color.a);
                surface.draw_set_text_pos(
                    (bbox.right + 10) as u32,
                    (bbox.top + (j as i32 * 10)) as u32,
                );
                surface.draw_print_text(cond);
            }
        }
    }
}

fn building_esp(
    entity: &Entity,
    name: &str,
    cfg: &EntityESPConfig,
    surface: &Surface,
    color: &RGBA,
) -> Option<BBox> {
    let Some(bbox) = helpers::get_bounding_box(entity) else {
        return None;
    };

    if cfg.boxes {
        draw_box(&bbox, color, surface);
    }

    if cfg.names {
        draw_name(&bbox, name, color, surface);
    }

    if cfg.health {
        draw_health(&bbox, entity, surface, true);
    }

    Some(bbox)
}

// TODO: Outline in black for visibility
fn draw_box(bbox: &BBox, color: &RGBA, surface: &Surface) {
    surface.draw_set_color(0, 0, 0, 255);
    surface.draw_outlined_rect(bbox.left - 1, bbox.top - 1, bbox.right + 1, bbox.bottom + 1);
    surface.draw_outlined_rect(bbox.left + 1, bbox.top + 1, bbox.right - 1, bbox.bottom - 1);
    surface.draw_set_color(color.r, color.g, color.b, color.a);
    surface.draw_outlined_rect(bbox.left, bbox.top, bbox.right, bbox.bottom);
}

// TODO: Add custom positioning
fn draw_name(bbox: &BBox, name: &str, color: &RGBA, surface: &Surface) {
    surface.draw_set_text_color(color.r, color.g, color.b, color.a);
    surface.draw_set_text_pos(bbox.left as u32, (bbox.top - 20) as u32);
    surface.draw_print_text(name);
}

// TODO: Add overheal + custom positioning
fn draw_health(bbox: &BBox, entity: &Entity, surface: &Surface, horizontal: bool) {
    let max_health = entity.max_health();
    if max_health <= 0 {
        return;
    }

    let height = bbox.bottom - bbox.top;
    let width = bbox.right - bbox.left;
    if height <= 0 || width <= 0 {
        return;
    }

    let health_raw = entity.health();
    let health = health_raw.clamp(0, max_health);

    let mut hp_pct = health as f32 / max_health as f32;
    if !hp_pct.is_finite() {
        return;
    }
    hp_pct = hp_pct.clamp(0.0, 1.0);

    // Color gradient
    let green = (hp_pct * 2.0 * 255.0).min(255.0).max(0.0) as i32;
    let red = ((1.0 - hp_pct) * 2.0 * 255.0).min(255.0).max(0.0) as i32;

    if horizontal {
        let bar_width = (width as f32 * hp_pct).round().clamp(0.0, width as f32) as i32;

        let bar_left = bbox.left;
        let bar_right = bbox.left + bar_width;
        let bar_top = bbox.bottom + 2;
        let bar_bot = bbox.bottom + 5;

        // Background
        surface.draw_set_color(0, 0, 0, 128);
        surface.draw_filled_rect(bbox.left, bar_top, bbox.right, bar_bot);

        // Health amount
        surface.draw_set_color(red, green, 0, 255);
        surface.draw_filled_rect(bar_left, bar_top + 1, bar_right, bar_bot - 1);
    } else {
        let bar_height = (height as f32 * hp_pct).round().clamp(0.0, height as f32) as i32;

        let bg_top = bbox.bottom - height;

        surface.draw_set_color(0, 0, 0, 128);
        surface.draw_filled_rect(bbox.right + 1, bg_top, bbox.right + 4, bbox.bottom);

        let bar_top = bbox.bottom - bar_height;

        surface.draw_set_color(red, green, 0, 255);
        surface.draw_filled_rect(bbox.right + 2, bar_top, bbox.right + 3, bbox.bottom - 1);
    }
}

// TODO: Fix for scoped weapons
pub fn draw_fov(surface: &Surface, config: &Config) {
    if !config.aimbot.master || !config.aimbot.draw_fov {
        return;
    }

    let fov = config.aimbot.fov as f32;
    let (width, height) = Interfaces::engine_client().get_screen_size();

    let radius = (f32::tan((fov / 2.0).to_radians()) / f32::tan(45.0f32.to_radians()))
        * (width as f32 / 2.0);

    surface.draw_set_color(255, 255, 255, 255);
    surface.draw_circle(width / 2, height / 2, radius as i32, 255);
}
