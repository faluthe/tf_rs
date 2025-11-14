use std::sync::OnceLock;

use log::info;

use crate::{
    cfg_enabled, cfg_get, globals::Globals, helpers, interfaces::Interfaces, types::Player,
};

static ESP_FONT: OnceLock<u64> = OnceLock::new();

pub fn esp_font() -> u64 {
    *ESP_FONT.get_or_init(|| {
        let font = Interfaces::surface().create_font();
        Interfaces::surface().set_font_glyph_set(font, "DejaVu Sans Mono", 14, 400, 0, 0, 0x0);
        font
    })
}

pub fn player_boxes(localplayer: &Player) {
    if !cfg_enabled!(esp) {
        return;
    }

    let globals = Globals::read();
    let target = globals.target.as_ref();

    Interfaces::surface().draw_set_color(255, 255, 255, 255);

    for i in 1..=Interfaces::engine_client().get_max_clients() {
        if let Some(player) = Interfaces::entity_list().get_client_entity::<Player>(i) {
            if &player == localplayer
                || player.is_dormant()
                || player.is_dead()
                || player.team() == localplayer.team()
            {
                continue;
            }

            if let Some((left, top, right, bottom)) = helpers::get_bounding_box(player) {
                Interfaces::surface().draw_outlined_rect(left, top, right, bottom);

                if Some(i) == target.map(|t| t.target_index) {
                    Interfaces::surface().draw_set_text_pos((right + 10) as u32, top as u32);
                    Interfaces::surface().draw_print_text("TARGET");
                    if Some(true) == target.map(|t| t.should_headshot) {
                        Interfaces::surface()
                            .draw_set_text_pos((right + 10) as u32, (top + 10) as u32);
                        Interfaces::surface().draw_print_text("HS");
                    }
                }
            }
        }
    }
}

pub fn draw_fov() {
    if !cfg_enabled!(aimbot_fov) {
        return;
    }

    let fov = cfg_get!(aimbot_fov) as f32;
    let (width, height) = Interfaces::engine_client().get_screen_size();

    let radius = (f32::tan((fov / 2.0).to_radians()) / f32::tan(45.0f32.to_radians()))
        * (width as f32 / 2.0);

    Interfaces::surface().draw_set_color(255, 255, 255, 255);
    Interfaces::surface().draw_circle(width / 2, height / 2, radius as i32, 255);
}
