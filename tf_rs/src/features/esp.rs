use std::sync::OnceLock;

use log::info;

use crate::{
    config::Config,
    globals::{Globals, Target},
    helpers,
    interfaces::Interfaces,
    types::{Entity, Player, entity::EntityClassID},
};

static ESP_FONT: OnceLock<u64> = OnceLock::new();

pub fn esp_font() -> u64 {
    *ESP_FONT.get_or_init(|| {
        let font = Interfaces::surface().create_font();
        Interfaces::surface().set_font_glyph_set(font, "DejaVu Sans Mono", 14, 400, 0, 0, 0x0);
        font
    })
}

pub fn player_esp(localplayer: &Player, config: &Config) {
    if config.esp.master == 0 {
        return;
    }

    let globals = Globals::read();
    let target = globals.target.as_ref();

    for i in 1..Interfaces::engine_client().get_max_clients() {
        if let Some(player) = Interfaces::entity_list().get_client_entity::<Player>(i) {
            if &player == localplayer
                || player.is_dormant()
                || player.is_dead()
                || player.team() == localplayer.team()
            // TODO: Add friendly ESP?
            {
                continue;
            }

            if let Some((left, top, right, bottom)) = helpers::get_bounding_box(&player) {
                draw_box(left, top, right, bottom, config);
                draw_name(left, top, right, bottom, i, config);
                draw_health(config, &player, top, bottom, right);

                if Some(i) == target.map(|t| t.target_index) {
                    draw_target(left, top, right, bottom, target, config);
                }
            }
        }
    }
}

pub fn entity_esp(_localplayer: &Player, config: &Config) {
    if config.esp.master == 0 {
        return;
    }

    for i in Interfaces::engine_client().get_max_clients()..Interfaces::entity_list().max_entities()
    {
        if let Some(entity) = Interfaces::entity_list().get_client_entity::<Entity>(i) {
            if entity.is_dormant() {
                continue;
            }

            match entity.class_id() {
                EntityClassID::Sentry | EntityClassID::Dispenser | EntityClassID::Teleporter => {
                    if let Some((left, top, right, bottom)) = helpers::get_bounding_box(&entity) {
                        draw_box(left, top, right, bottom, config);
                        draw_health(config, &entity, top, bottom, right);
                    }
                }
                _ => {}
            }
        }
    }
}

fn draw_box(left: i32, top: i32, right: i32, bottom: i32, config: &Config) {
    if config.esp.boxes == 0 {
        return;
    }

    Interfaces::surface().draw_set_color(255, 255, 255, 255);
    Interfaces::surface().draw_outlined_rect(left, top, right, bottom);
}

fn draw_name(left: i32, top: i32, _right: i32, _bottom: i32, player_index: i32, config: &Config) {
    if config.esp.names == 0 {
        return;
    }

    let name = Interfaces::engine_client()
        .get_player_info(player_index)
        .name;
    let name = str::from_utf8(&name).unwrap_or("");

    Interfaces::surface().draw_set_text_color(255, 255, 255, 255);
    Interfaces::surface().draw_set_text_pos(left as u32, (top - 20) as u32);
    Interfaces::surface().draw_print_text(name);
}

// TODO: Add overheal
fn draw_health(config: &Config, player: &Entity, top: i32, bottom: i32, right: i32) {
    if config.esp.health == 0 {
        return;
    }

    let max_health = player.max_health();

    info!("max_health: {}", max_health);

    if max_health <= 0 {
        return;
    }

    let height = bottom - top;
    if height <= 0 {
        return;
    }

    let health_raw = player.health();
    info!("health: {}", health_raw);
    let health = health_raw.clamp(0, max_health);

    let mut health_percent = health as f32 / max_health as f32;
    if !health_percent.is_finite() {
        return;
    }
    health_percent = health_percent.clamp(0.0, 1.0);

    let bar_height = (height as f32 * health_percent)
        .clamp(0.0, height as f32)
        .round() as i32;

    let bg_top = bottom - height;
    if bg_top >= bottom {
        return;
    }

    Interfaces::surface().draw_set_color(0, 0, 0, 255 / 2);
    Interfaces::surface().draw_filled_rect(right + 1, bg_top, right + 4, bottom);

    let green = (health_percent * 2.0 * 255.0).min(255.0).max(0.0) as i32;
    let red = ((1.0 - health_percent) * 2.0 * 255.0).min(255.0).max(0.0) as i32;

    let bar_top = bottom - bar_height;
    if bar_top >= bottom {
        return;
    }

    Interfaces::surface().draw_set_color(red, green, 0, 255);
    Interfaces::surface().draw_filled_rect(right + 2, bar_top, right + 3, bottom - 1);
}

fn draw_target(
    _left: i32,
    top: i32,
    right: i32,
    _bottom: i32,
    target: Option<&Target>,
    config: &Config,
) {
    if config.esp.aimbot_target == 0 {
        return;
    }

    Interfaces::surface().draw_set_text_color(255, 255, 255, 255);
    Interfaces::surface().draw_set_text_pos((right + 10) as u32, top as u32);
    Interfaces::surface().draw_print_text("TARGET");

    if Some(true) == target.map(|t| t.should_headshot) {
        Interfaces::surface().draw_set_text_pos((right + 10) as u32, (top + 10) as u32);
        Interfaces::surface().draw_print_text("HS");
    }
}

// TODO: Fix for scoped weapons
pub fn draw_fov(config: &Config) {
    if config.aimbot.draw_fov == 0 {
        return;
    }

    let fov = config.aimbot.fov as f32;
    let (width, height) = Interfaces::engine_client().get_screen_size();

    let radius = (f32::tan((fov / 2.0).to_radians()) / f32::tan(45.0f32.to_radians()))
        * (width as f32 / 2.0);

    Interfaces::surface().draw_set_color(255, 255, 255, 255);
    Interfaces::surface().draw_circle(width / 2, height / 2, radius as i32, 255);
}
