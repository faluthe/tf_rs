use nuklear::{
    Nuklear, Rect,
    flags::{PanelFlags, TextAlignment},
};

use crate::{
    config::Config,
    interfaces::Interfaces,
    types::{Player, rgba},
};

pub fn draw(localplayer: &Player, config: &Config, nk: &Nuklear) {
    if !config.spectator_list {
        return;
    }

    let mut spectators = Vec::new();
    let local = if localplayer.is_dead() {
        localplayer.observer_target()
    } else {
        Some(localplayer.ent)
    };

    let Some(local) = local else {
        return;
    };

    for i in 1..Interfaces::engine_client().get_max_clients() {
        if let Some(player) = Interfaces::entity_list().get_client_entity::<Player>(i) {
            if player.is_dormant() || player.ent == local || !player.is_dead() {
                continue;
            }

            let (observer_mode, color) = match player.observer_mode() {
                1 => ("Deathcam", &rgba::WHITE),
                2 => ("Freecam", &rgba::WHITE),
                3 => ("Fixed", &rgba::WHITE),
                4 => ("Firstperson", &rgba::RED),
                5 => ("Thirdperson", &rgba::WHITE),
                6 => ("Point of Interest", &rgba::WHITE),
                7 => ("Roaming", &rgba::WHITE),
                _ => ("Unknown", &rgba::WHITE),
            };

            let Some(obs_target) = player.observer_target() else {
                continue;
            };

            if obs_target != local {
                continue;
            }

            let name = Interfaces::engine_client().get_player_info(i).name;
            let spectator = format!(
                "[{}] {}",
                observer_mode,
                str::from_utf8(&name).unwrap_or("").trim_end_matches('\0')
            );

            spectators.push((spectator, color));
        }
    }

    if spectators.is_empty() {
        return;
    }

    let (screen_w, screen_h) = Interfaces::engine_client().get_screen_size();
    let row_h = 20.0;
    let w = 300.0;
    let h = spectators.len() as f32 * row_h + 45.0;
    let x = screen_w as f32 - w;
    let y = screen_h as f32 / 3.0;

    if nk.begin(
        "Spectators",
        PanelFlags::TITLE | PanelFlags::NO_SCROLLBAR,
        Rect { x, y, w, h },
    ) {
        for (spec, color) in spectators {
            nk.row_dynamic(20.0, 1).colored_label(
                spec.as_str(),
                TextAlignment::LEFT,
                color.r as u8,
                color.g as u8,
                color.b as u8,
                color.a as u8,
            );
        }
    }
    nk.end();
}
