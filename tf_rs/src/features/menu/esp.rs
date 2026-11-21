use std::sync::atomic::{AtomicUsize, Ordering};

use nuklear::{
    Color, Nuklear,
    flags::{LayoutFormat, TextAlignment},
};

use super::{ROW, field};
use crate::{
    config::{ChamsConfig, ChamsGroupConfig, Config, EntityESPConfig},
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

/// Chams tab shown in the menu, index into `CHAMS_TABS`
static CHAMS_TAB: AtomicUsize = AtomicUsize::new(0);
const CHAMS_TABS: [&str; 4] = ["Players", "Buildings", "Categories", "First person"];

fn chams_section(nk: &Nuklear, chams: &mut ChamsConfig) {
    nk.section("Chams", false)
        .row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut chams.enabled);
    if !chams.enabled {
        return;
    }

    let mut tab = CHAMS_TAB.load(Ordering::Relaxed);
    nk.tab_bar(&CHAMS_TABS, &mut tab);
    CHAMS_TAB.store(tab, Ordering::Relaxed);

    match tab {
        1 => chams_group(nk, "Buildings", &mut chams.buildings),
        2 => {
            let names = player_db::get_names();
            chams_group(nk, &names[0], &mut chams.category1);
            chams_group(nk, &names[1], &mut chams.category2);
            chams_group(nk, &names[2], &mut chams.category3);
            chams_group(nk, &names[3], &mut chams.category4);
        }
        3 => {
            first_person_group(nk, "Hands", &mut chams.hands);
            first_person_group(nk, "Weapon", &mut chams.weapon);
        }
        _ => {
            chams_group(nk, "Enemies", &mut chams.enemy);
            chams_group(nk, "Teammates", &mut chams.friendly);
            chams_group(nk, "Local player", &mut chams.local);
        }
    }
}

fn chams_group(nk: &Nuklear, title: &str, group: &mut ChamsGroupConfig) {
    group_title(nk, title);
    material_field(nk, &mut group.material);
    toggle_color_field(nk, "Visible", &mut group.visible, &mut group.visible_color);
    toggle_color_field(
        nk,
        "Occluded",
        &mut group.occluded,
        &mut group.occluded_color,
    );
}

// Viewmodels are never behind anything, so only the visible pass applies
fn first_person_group(nk: &Nuklear, title: &str, group: &mut ChamsGroupConfig) {
    group_title(nk, title);
    material_field(nk, &mut group.material);
    toggle_color_field(nk, "Enabled", &mut group.visible, &mut group.visible_color);
}

fn group_title(nk: &Nuklear, title: &str) {
    nk.row_dynamic(ROW, 1);
    match nk.palette() {
        Some(p) => nk.colored_label(
            title,
            TextAlignment::LEFT,
            p.accent.r,
            p.accent.g,
            p.accent.b,
            p.accent.a,
        ),
        None => nk.label(title, TextAlignment::LEFT),
    };
}

fn material_field(nk: &Nuklear, material: &mut usize) {
    field(nk, "Material", |nk| {
        let labels: Vec<&str> = chams::MATERIALS.iter().map(|m| m.label).collect();
        nk.single_select_combo(&labels, material);
    });
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
