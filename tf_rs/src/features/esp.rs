use std::sync::OnceLock;

use crate::{
    cfg_enabled, cfg_get,
    globals::{Globals, Target},
    helpers,
    interfaces::Interfaces,
    types::Player,
};

static ESP_FONT: OnceLock<u64> = OnceLock::new();

pub fn esp_font() -> u64 {
    *ESP_FONT.get_or_init(|| {
        let font = Interfaces::surface().create_font();
        Interfaces::surface().set_font_glyph_set(font, "DejaVu Sans Mono", 14, 400, 0, 0, 0x0);
        font
    })
}

pub fn player_esp(localplayer: &Player) {
    if !cfg_enabled!(esp) {
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

            if let Some((left, top, right, bottom)) = helpers::get_bounding_box(player) {
                draw_box(left, top, right, bottom);
                draw_name(left, top, right, bottom, i);

                if Some(i) == target.map(|t| t.target_index) {
                    draw_target(left, top, right, bottom, target);
                }
            }
        }
    }
}

fn draw_box(left: i32, top: i32, right: i32, bottom: i32) {
    if !cfg_enabled!(esp_boxes) {
        return;
    }

    Interfaces::surface().draw_set_color(255, 255, 255, 255);
    Interfaces::surface().draw_outlined_rect(left, top, right, bottom);
}

fn draw_name(left: i32, top: i32, _right: i32, _bottom: i32, player_index: i32) {
    if !cfg_enabled!(esp_names) {
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

fn draw_target(_left: i32, top: i32, right: i32, _bottom: i32, target: Option<&Target>) {
    if !cfg_enabled!(esp_aimbot_target) {
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
pub fn draw_fov() {
    if !cfg_enabled!(draw_fov) {
        return;
    }

    let fov = cfg_get!(aimbot_fov) as f32;
    let (width, height) = Interfaces::engine_client().get_screen_size();

    let radius = (f32::tan((fov / 2.0).to_radians()) / f32::tan(45.0f32.to_radians()))
        * (width as f32 / 2.0);

    Interfaces::surface().draw_set_color(255, 255, 255, 255);
    Interfaces::surface().draw_circle(width / 2, height / 2, radius as i32, 255);
}
