use nuklear::Nuklear;

use super::{ROW, field};
use crate::{
    config::{ChamsGroupConfig, Config, EntityESPConfig},
    features::chams,
    player_db,
};

const PICKER: f32 = 180.0;

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

        let c = &mut esp.chams;
        let names = player_db::get_names();
        chams_group(nk, "Enemies", &mut c.enemy);
        chams_group(nk, "Teammates", &mut c.friendly);
        chams_group(nk, "Local player", &mut c.local);
        chams_group(nk, "Buildings", &mut c.buildings);
        chams_group(nk, &names[0], &mut c.category1);
        chams_group(nk, &names[1], &mut c.category2);
        chams_group(nk, &names[2], &mut c.category3);
        chams_group(nk, &names[3], &mut c.category4);
        first_person_group(nk, "Hands", &mut c.hands);
        first_person_group(nk, "Weapon", &mut c.weapon);
    }
}

fn chams_group(nk: &Nuklear, title: &str, group: &mut ChamsGroupConfig) {
    if !nk.tree_push(title) {
        return;
    }
    nk.row_dynamic(ROW, 2)
        .checkbox("Visible", &mut group.visible)
        .checkbox("Occluded", &mut group.occluded);
    nk.row_dynamic(PICKER, 2)
        .color_picker(&mut group.visible_color)
        .color_picker(&mut group.occluded_color);
    nk.tree_pop();
}

// Viewmodels are never behind anything, so only the visible pass applies
fn first_person_group(nk: &Nuklear, title: &str, group: &mut ChamsGroupConfig) {
    if !nk.tree_push(title) {
        return;
    }
    nk.row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut group.visible);
    nk.row_dynamic(PICKER, 1)
        .color_picker(&mut group.visible_color);
    nk.tree_pop();
}
