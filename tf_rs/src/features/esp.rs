use std::sync::OnceLock;

use crate::{
    config::Config,
    helpers,
    interfaces::Interfaces,
    types::{BBox, ClassID, Entity, Player, RGBA},
};

static ESP_FONT: OnceLock<u64> = OnceLock::new();

pub fn esp_font() -> u64 {
    *ESP_FONT.get_or_init(|| {
        let font = Interfaces::surface().create_font();
        Interfaces::surface().set_font_glyph_set(font, "DejaVu Sans Mono", 14, 400, 0, 0, 0x0);
        font
    })
}

pub fn run(localplayer: &Player, config: &Config) {
    if config.esp.master == 0 {
        return;
    }

    // let globals = Globals::read();
    // let target = globals.target.as_ref(); // TODO: Add this back lol but smarter

    for i in 1..Interfaces::entity_list().max_entities() {
        if let Some(entity) = Interfaces::entity_list().get_client_entity::<Entity>(i) {
            if entity.is_dormant() || entity == localplayer.ent {
                continue;
            }

            match entity.class_id() {
                ClassID::Player => {
                    let Some(bbox) = helpers::get_bounding_box(&entity) else {
                        continue;
                    };

                    let team_color = entity.team().as_rgba();

                    if config.esp.player_boxes != 0 {
                        draw_box(&bbox, &team_color);
                    }
                    if config.esp.player_names != 0 {
                        let name = Interfaces::engine_client().get_player_info(i).name;
                        let name = str::from_utf8(&name).unwrap_or("");

                        draw_name(&bbox, name, &team_color);
                    }
                    if config.esp.player_health != 0 {
                        draw_health(&bbox, &entity);
                    }
                }
                ClassID::Sentry => building_esp(&entity, "Sentry", config),
                ClassID::Dispenser => building_esp(&entity, "Dispenser", config),
                ClassID::Teleporter => building_esp(&entity, "Teleporter", config),
                _ => {}
            }
        }
    }
}

fn building_esp(entity: &Entity, name: &str, config: &Config) {
    let Some(bbox) = helpers::get_bounding_box(entity) else {
        return;
    };

    let team_color = entity.team().as_rgba();

    if config.esp.building_boxes != 0 {
        draw_box(&bbox, &team_color);
    }
    if config.esp.building_names != 0 {
        draw_name(&bbox, name, &team_color);
    }
    if config.esp.building_health != 0 {
        draw_health(&bbox, entity);
    }
}

// TODO: Outline in black for visibility
fn draw_box(bbox: &BBox, color: &RGBA) {
    Interfaces::surface().draw_set_color(color.r, color.g, color.b, color.a);
    Interfaces::surface().draw_outlined_rect(bbox.left, bbox.top, bbox.right, bbox.bottom);
}

// TODO: Add custom positioning
fn draw_name(bbox: &BBox, name: &str, color: &RGBA) {
    Interfaces::surface().draw_set_text_color(color.r, color.g, color.b, color.a);
    Interfaces::surface().draw_set_text_pos(bbox.left as u32, (bbox.top - 20) as u32);
    Interfaces::surface().draw_print_text(name);
}

// TODO: Add overheal + custom positioning
fn draw_health(bbox: &BBox, entity: &Entity) {
    let max_health = entity.max_health();
    if max_health <= 0 {
        return;
    }

    let height = bbox.bottom - bbox.top;
    if height <= 0 {
        return;
    }

    let health_raw = entity.health();
    let health = health_raw.clamp(0, max_health);

    let mut health_percent = health as f32 / max_health as f32;
    if !health_percent.is_finite() {
        return;
    }
    health_percent = health_percent.clamp(0.0, 1.0);

    let bar_height = (height as f32 * health_percent)
        .clamp(0.0, height as f32)
        .round() as i32;

    let bg_top = bbox.bottom - height;
    if bg_top >= bbox.bottom {
        return;
    }

    Interfaces::surface().draw_set_color(0, 0, 0, 255 / 2);
    Interfaces::surface().draw_filled_rect(bbox.right + 1, bg_top, bbox.right + 4, bbox.bottom);

    let green = (health_percent * 2.0 * 255.0).min(255.0).max(0.0) as i32;
    let red = ((1.0 - health_percent) * 2.0 * 255.0).min(255.0).max(0.0) as i32;

    let bar_top = bbox.bottom - bar_height;
    if bar_top >= bbox.bottom {
        return;
    }

    Interfaces::surface().draw_set_color(red, green, 0, 255);
    Interfaces::surface().draw_filled_rect(
        bbox.right + 2,
        bar_top,
        bbox.right + 3,
        bbox.bottom - 1,
    );
}

// fn draw_target(
//     _left: i32,
//     top: i32,
//     right: i32,
//     _bottom: i32,
//     target: Option<&Target>,
//     config: &Config,
// ) {
//     if config.esp.aimbot_target == 0 {
//         return;
//     }

//     Interfaces::surface().draw_set_text_color(255, 255, 255, 255);
//     Interfaces::surface().draw_set_text_pos((right + 10) as u32, top as u32);
//     Interfaces::surface().draw_print_text("TARGET");

//     if Some(true) == target.map(|t| t.should_headshot) {
//         Interfaces::surface().draw_set_text_pos((right + 10) as u32, (top + 10) as u32);
//         Interfaces::surface().draw_print_text("HS");
//     }
// }

// TODO: Fix for scoped weapons
pub fn draw_fov(config: &Config) {
    if config.aimbot.master == 0 || config.aimbot.draw_fov == 0 {
        return;
    }

    let fov = config.aimbot.fov as f32;
    let (width, height) = Interfaces::engine_client().get_screen_size();

    let radius = (f32::tan((fov / 2.0).to_radians()) / f32::tan(45.0f32.to_radians()))
        * (width as f32 / 2.0);

    Interfaces::surface().draw_set_color(255, 255, 255, 255);
    Interfaces::surface().draw_circle(width / 2, height / 2, radius as i32, 255);
}
