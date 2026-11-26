use std::sync::RwLock;

use nuklear::{
    Nuklear,
    flags::{EditFlags, LayoutFormat, PanelFlags, TextAlignment},
};

use super::ROW;
use crate::{
    config::{Config, PlayerCategoryColors},
    interfaces::Interfaces,
    player_db,
    types::Player,
};

static CAT_NAME_BUFS: [RwLock<[u8; 64]>; 4] = [
    RwLock::new([0u8; 64]),
    RwLock::new([0u8; 64]),
    RwLock::new([0u8; 64]),
    RwLock::new([0u8; 64]),
];
static CAT_NAMES_BUF_INIT: std::sync::Once = std::sync::Once::new();

pub(super) fn players_tab(nk: &Nuklear, config: &mut Config) {
    CAT_NAMES_BUF_INIT.call_once(|| {
        let names = player_db::get_names();
        for (i, name) in names.iter().enumerate() {
            let mut buf = CAT_NAME_BUFS[i].write().unwrap();
            let bytes = name.as_bytes();
            let len = bytes.len().min(63);
            buf[..len].copy_from_slice(&bytes[..len]);
            buf[len] = 0;
        }
    });

    nk.section("Players", true);
    player_list(nk);
    category_names(nk, &mut config.colors.player_categories);
}

fn player_list(nk: &Nuklear) {
    let engine = Interfaces::engine_client();
    if !engine.is_in_game() {
        nk.row_dynamic(ROW, 1)
            .dim_label("Not in a game.", TextAlignment::LEFT);
        return;
    }

    nk.layout_row_begin(LayoutFormat::DYNAMIC, 20.0, 3)
        .layout_row_push(0.30)
        .dim_label("NAME", TextAlignment::LEFT)
        .layout_row_push(0.45)
        .dim_label("GUID", TextAlignment::LEFT)
        .layout_row_push(0.25)
        .dim_label("CATEGORY", TextAlignment::LEFT)
        .layout_row_end();

    // Collect players grouped by team name
    let mut teams: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for i in 1..=engine.get_max_clients() {
        let info = engine.get_player_info(i);
        let name = str::from_utf8(&info.name)
            .unwrap_or("")
            .trim_end_matches('\0')
            .to_owned();
        if name.is_empty() {
            continue;
        }
        let guid = str::from_utf8(&info.guid)
            .unwrap_or("")
            .trim_end_matches('\0')
            .to_owned();
        let team_name = Interfaces::entity_list()
            .get_client_entity::<Player>(i)
            .map(|p| format!("{:?}", p.team()))
            .unwrap_or_else(|| "Unknown".to_owned());
        if let Some(entry) = teams.iter_mut().find(|(t, _)| t == &team_name) {
            entry.1.push((name, guid));
        } else {
            teams.push((team_name, vec![(name, guid)]));
        }
    }
    teams.sort_by(|a, b| a.0.cmp(&b.0));

    // Leave room for the category name editor below
    let (_, height) = nk.get_content_region();
    nk.row_dynamic((height - 260.0).max(150.0), 1);
    if nk.group_begin("Players", PanelFlags::BORDER) {
        for (team_name, players) in &teams {
            let (r, g, b) = match team_name.as_str() {
                "Red" => (255u8, 80u8, 80u8),
                "Blue" => (100u8, 160u8, 255u8),
                _ => (200u8, 200u8, 200u8),
            };
            nk.row_dynamic(22.0, 1).colored_label(
                team_name.to_uppercase(),
                TextAlignment::LEFT,
                r,
                g,
                b,
                255,
            );
            nk.horizontal_separator(1.0);

            for (name, guid) in players {
                let cat = player_db::get(guid.as_str());
                let cat_display = cat.display_name();

                nk.layout_row_begin(LayoutFormat::DYNAMIC, 22.0, 3)
                    .layout_row_push(0.30);
                nk.label(name.as_str(), TextAlignment::LEFT);
                nk.layout_row_push(0.45);
                nk.dim_label(guid.as_str(), TextAlignment::LEFT);
                nk.layout_row_push(0.25);
                if nk.button_label(cat_display.as_str()) {
                    player_db::set(guid.as_str(), cat.next());
                }
                nk.layout_row_end();
            }
        }
        nk.group_end();
    }
}

fn category_names(nk: &Nuklear, colors: &mut PlayerCategoryColors) {
    nk.section("Categories", false);
    let colors = [
        &mut colors.category1,
        &mut colors.category2,
        &mut colors.category3,
        &mut colors.category4,
    ];
    for (buf, color) in CAT_NAME_BUFS.iter().zip(colors) {
        nk.layout_row_begin(LayoutFormat::DYNAMIC, ROW, 2)
            .layout_row_push(0.75);
        let mut buf = buf.write().unwrap();
        nk.edit_string(EditFlags::EDIT_FIELD, buf.as_mut_ptr() as *mut i8, 64);
        nk.layout_row_push(0.25).color_combo(color);
        nk.layout_row_end();
    }
    nk.row_dynamic(ROW, 1);
    if nk.button_label("Save names") {
        for (i, buf) in CAT_NAME_BUFS.iter().enumerate() {
            let buf = buf.read().unwrap();
            let end = buf.iter().position(|&c| c == 0).unwrap_or(64);
            let name = str::from_utf8(&buf[..end]).unwrap_or("");
            player_db::set_name(i, name);
        }
    }
}
