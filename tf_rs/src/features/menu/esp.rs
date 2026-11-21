use std::sync::atomic::{AtomicUsize, Ordering};

use nuklear::{
    Color, Nuklear,
    flags::{LayoutFormat, TextAlignment},
};

use super::{ROW, field};
use crate::{
    config::{ChamsConfig, Config, EntityESPConfig},
    features::chams,
    player_db,
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

    chams_section(nk, &mut esp.chams);
}

/// Chams group shown in the menu, index into the labels in `chams_section`
static CHAMS_GROUP: AtomicUsize = AtomicUsize::new(0);

fn chams_section(nk: &Nuklear, chams: &mut ChamsConfig) {
    nk.section("Chams", false)
        .row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut chams.enabled);
    if !chams.enabled {
        return;
    }

    let names = player_db::get_names();
    let labels = [
        "Enemies",
        "Teammates",
        "Local player",
        "Buildings",
        names[0].as_str(),
        names[1].as_str(),
        names[2].as_str(),
        names[3].as_str(),
        "Hands",
        "Weapon",
    ];
    let mut selected = CHAMS_GROUP.load(Ordering::Relaxed);
    field(nk, "Group", |nk| {
        nk.single_select_combo(&labels, &mut selected);
    });
    CHAMS_GROUP.store(selected, Ordering::Relaxed);

    let (group, first_person) = match selected {
        1 => (&mut chams.friendly, false),
        2 => (&mut chams.local, false),
        3 => (&mut chams.buildings, false),
        4 => (&mut chams.category1, false),
        5 => (&mut chams.category2, false),
        6 => (&mut chams.category3, false),
        7 => (&mut chams.category4, false),
        8 => (&mut chams.hands, true),
        9 => (&mut chams.weapon, true),
        _ => (&mut chams.enemy, false),
    };

    field(nk, "Material", |nk| {
        let labels: Vec<&str> = chams::MATERIALS.iter().map(|m| m.label).collect();
        nk.single_select_combo(&labels, &mut group.material);
    });

    // Viewmodels are never behind anything, so only the visible pass applies
    if first_person {
        toggle_color_field(nk, "Enabled", &mut group.visible, &mut group.visible_color);
    } else {
        toggle_color_field(nk, "Visible", &mut group.visible, &mut group.visible_color);
        toggle_color_field(
            nk,
            "Occluded",
            &mut group.occluded,
            &mut group.occluded_color,
        );
    }
}

/// A `field` row whose widget is a checkbox followed by a color swatch
fn toggle_color_field(nk: &Nuklear, label: &str, on: &mut bool, color: &mut Color) {
    nk.layout_row_begin(LayoutFormat::DYNAMIC, ROW, 3)
        .layout_row_push(0.45)
        .label(label, TextAlignment::LEFT)
        .layout_row_push(0.1)
        .checkbox("", on)
        .layout_row_push(0.45)
        .color_combo(color);
    nk.layout_row_end();
}
