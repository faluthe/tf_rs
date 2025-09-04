use crate::{helpers, interfaces::Interfaces};

pub fn player_boxes() {
    if !Interfaces::engine_client().is_in_game() {
        return;
    }

    let localplayer = helpers::get_localplayer().expect("Failed to get localplayer");

    Interfaces::surface().draw_set_color(255, 255, 255, 255);

    for i in 1..=Interfaces::engine_client().get_max_clients() {
        if let Some(player) = Interfaces::entity_list().get_client_entity(i) {
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
