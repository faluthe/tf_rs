use std::sync::OnceLock;

use crate::{
    config::{Config, EntityESPConfig},
    globals::Globals,
    helpers,
    interfaces::{Interfaces, Surface},
    types::{BBox, ClassId, Cond, Entity, Player, RGBA, rgba},
};

static ESP_FONT: OnceLock<u64> = OnceLock::new();

pub fn esp_font(surface: &Surface) -> u64 {
    *ESP_FONT.get_or_init(|| {
        let font = surface.create_font();
        surface.set_font_glyph_set(font, "DejaVu Sans Mono", 14, 400, 0, 0, 0x0);
        font
    })
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

            let mut conds = Vec::new();

            let is_target = if config.esp.aimbot_target {
                if Some(i) == target.map(|t| t.target_index) {
                    conds.push(("TARGET", &rgba::WHITE));

                    if Some(true) == target.map(|t| t.should_headshot) {
                        conds.push(("HS", &rgba::RED));
                    }
                    true
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
                            conds.push(("DISGUISED", &rgba::WHITE));
                        }

                        if player.in_cond(Cond::Taunting) {
                            conds.push(("TAUNTING", &rgba::WHITE));
                        }

                        if player.in_cond(Cond::Zoomed) {
                            conds.push(("ZOOMED", &rgba::WHITE));
                        }

                        if player.is_invisible() {
                            conds.push(("INVISIBLE", &rgba::WHITE));
                        }

                        if player.in_cond(Cond::MadMilk) {
                            conds.push(("MILKED", &rgba::WHITE));
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
    if config.aimbot.master || config.aimbot.draw_fov {
        return;
    }

    let fov = config.aimbot.fov as f32;
    let (width, height) = Interfaces::engine_client().get_screen_size();

    let radius = (f32::tan((fov / 2.0).to_radians()) / f32::tan(45.0f32.to_radians()))
        * (width as f32 / 2.0);

    surface.draw_set_color(255, 255, 255, 255);
    surface.draw_circle(width / 2, height / 2, radius as i32, 255);
}
