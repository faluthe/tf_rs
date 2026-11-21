use nuklear::{Nuklear, flags::TextAlignment};

use super::{ROW, field};
use crate::{
    config::{Config, EntityESPConfig},
    features::chams,
};

pub(super) fn esp_tab(nk: &Nuklear, config: &mut Config) {
    fn entity_esp_combo(nk: &Nuklear, title: &str, esp_cfg: &mut EntityESPConfig) {
        field(nk, title, |nk| {
            nk.multi_select_combo(
                &["Boxes", "Names", "Health bar"],
                &mut [&mut esp_cfg.boxes, &mut esp_cfg.names, &mut esp_cfg.health],
            );
        });
    }

    let esp = &mut config.esp;

    nk.section("ESP", true)
        .row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut esp.master);

    if !esp.master {
        return;
    }

    nk.section("Players", false);
    entity_esp_combo(nk, "Enemies", &mut esp.player_enemy);
    entity_esp_combo(nk, "Teammates", &mut esp.player_friendly);
    field(nk, "Conditions", |nk| {
        nk.multi_select_combo(
            &[
                "Disguised",
                "Taunting",
                "Zoomed",
                "Invisible",
                "Milked",
                "No MG",
                "Butter",
                "Category",
            ],
            &mut [
                &mut esp.conds.disguised.enabled,
                &mut esp.conds.taunting.enabled,
                &mut esp.conds.zoomed.enabled,
                &mut esp.conds.invisible.enabled,
                &mut esp.conds.milked.enabled,
                &mut esp.conds.mg.enabled,
                &mut esp.conds.butter.enabled,
                &mut esp.conds.category,
            ],
        );
    });

    nk.section("Buildings", false);
    entity_esp_combo(nk, "Enemy", &mut esp.building_enemy);
    entity_esp_combo(nk, "Friendly", &mut esp.building_friendly);

    nk.section("Other", false)
        .row_dynamic(ROW, 1)
        .checkbox("Show aimbot target", &mut esp.aimbot_target)
        .checkbox("Ignore invisible", &mut esp.ignore_invis);

    nk.section("Chams", false)
        .row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut esp.chams.enabled);
    if esp.chams.enabled {
        field(nk, "Material", |nk| {
            let labels: Vec<&str> = chams::MATERIALS.iter().map(|(label, _)| *label).collect();
            nk.single_select_combo(&labels, &mut esp.chams.material);
        });
        nk.row_dynamic(ROW, 2)
            .dim_label("Visible", TextAlignment::LEFT)
            .dim_label("Occluded", TextAlignment::LEFT);
        nk.row_dynamic(180.0, 2)
            .color_picker(&mut esp.chams.visible)
            .color_picker(&mut esp.chams.occluded);
    }
}
