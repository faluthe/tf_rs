use std::sync::OnceLock;

use crate::{cfg_enabled, helpers, interfaces::Interfaces, types::Player};

static ESP_FONT: OnceLock<u64> = OnceLock::new();

pub fn esp_font() -> u64 {
    *ESP_FONT.get_or_init(|| {
        let font = Interfaces::surface().create_font();
        Interfaces::surface().set_font_glyph_set(font, "DejaVu Sans Mono", 14, 400, 0, 0, 0x0);
        font
    })
}

pub fn player_boxes() {
    if !cfg_enabled!(esp) || !Interfaces::engine_client().is_in_game() {
        return;
    }

    let localplayer = helpers::get_localplayer().expect("Failed to get localplayer");

    Interfaces::surface().draw_set_color(255, 255, 255, 255);

    for i in 1..=Interfaces::engine_client().get_max_clients() {
        if let Some(player) = Interfaces::entity_list().get_client_entity::<Player>(i) {
            if player == localplayer
                || player.is_dormant()
                || player.is_dead()
                || player.team() == localplayer.team()
            {
                continue;
            }

            if let Some((left, top, right, bottom)) = helpers::get_bounding_box(player) {
                Interfaces::surface().draw_outlined_rect(left, top, right, bottom);
            }
        }
    }
}
