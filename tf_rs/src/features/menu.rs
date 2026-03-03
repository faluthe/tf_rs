use std::sync::RwLock;

use nuklear::{
    Nuklear, Rect,
    flags::{EditFlags, LayoutFormat, PanelFlags, TextAlignment},
};

use crate::{
    config::{Config, EntityESPConfig},
    globals::Globals,
    interfaces::Interfaces,
    player_db,
    types::rgba,
};

// TODO: Clean this up lol
static mut TAB: MenuTab = MenuTab::Aimbot;
static mut SELECTED_CONFIG: usize = 0;
static NEW_CONFIG_NAME: RwLock<[u8; 256]> = RwLock::new([0; 256]);

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuTab {
    Aimbot,
    ESP,
    Colors,
    Misc,
    Config,
    Players,
}

pub fn draw(nk: &Nuklear) {
    if nk.begin(
        "TF_RS",
        PanelFlags::BORDER | PanelFlags::MOVABLE | PanelFlags::TITLE,
        Rect {
            x: 200.0,
            y: 200.0,
            w: 700.0,
            h: 500.0,
        },
    ) {
        let mut config = Config::write();

        nk.row_dynamic(30.0, 6);
        tab_button(nk, "Aimbot", MenuTab::Aimbot);
        tab_button(nk, "ESP", MenuTab::ESP);
        tab_button(nk, "Colors", MenuTab::Colors);
        tab_button(nk, "Misc", MenuTab::Misc);
        tab_button(nk, "Config", MenuTab::Config);
        tab_button(nk, "Players", MenuTab::Players);

        match unsafe { TAB } {
            MenuTab::Aimbot => aimbot_tab(nk, &mut config),
            MenuTab::ESP => esp_tab(nk, &mut config),
            MenuTab::Colors => colors_tab(nk, &mut config),
            MenuTab::Misc => misc_tab(nk, &mut config),
            MenuTab::Config => config_tab(nk, &mut config),
            MenuTab::Players => players_tab(nk),
        }
    }
    nk.end();
}

fn tab_button(nk: &Nuklear, title: &str, tab: MenuTab) {
    let color = if tab == unsafe { TAB } {
        &rgba::DARK_GREY
    } else {
        &rgba::LIGHT_GREY
    };

    nk.set_button_normal_color(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    );
    nk.set_button_rounding(0.0);

    if nk.button_label(title) {
        unsafe {
            TAB = tab;
        }
    }

    // Restore button default styles
    nk.set_button_normal_color(
        (rgba::LIGHT_GREY.r * 255.0) as u8,
        (rgba::LIGHT_GREY.g * 255.0) as u8,
        (rgba::LIGHT_GREY.b * 255.0) as u8,
        (rgba::LIGHT_GREY.a * 255.0) as u8,
    );
    nk.set_button_rounding(4.0);
}

fn aimbot_tab(nk: &Nuklear, config: &mut Config) {
    nk.row_dynamic(30.0, 1)
        .checkbox("Master", &mut config.aimbot.master);

    if config.aimbot.master {
        nk.row_dynamic(30.0, 1)
            .checkbox("Silent aim", &mut config.aimbot.silent_aim)
            .row_dynamic(30.0, 2)
            .checkbox("Use key", &mut config.aimbot.key.use_key);

        if config.aimbot.key.use_key {
            let mut g = Globals::write();
            let label = if g.aimbot_key_editing {
                "Press a key...".to_string()
            } else if config.aimbot.key.is_mouse_button {
                format!("Aimbot key: Mouse {}", config.aimbot.key.code as u32)
            } else {
                format!("Aimbot key: {}", config.aimbot.key.code as u32)
            };

            if nk.button_label(label.as_str()) {
                g.aimbot_key_editing = !g.aimbot_key_editing;
            }
        } else {
            nk.label("", TextAlignment::LEFT);
        }

        nk.row_dynamic(30.0, 1)
            .checkbox("Building aim", &mut config.aimbot.building_aim)
            .row_dynamic(30.0, 2)
            .label(
                format!("Aimbot FOV: {}", config.aimbot.fov),
                TextAlignment::LEFT,
            )
            .slider_int(1, &mut config.aimbot.fov, 100, 1)
            .row_dynamic(30.0, 1)
            .checkbox("Draw FOV", &mut config.aimbot.draw_fov);
    }
}

fn esp_tab(nk: &Nuklear, config: &mut Config) {
    fn entity_esp_combo(nk: &Nuklear, title: &str, esp_cfg: &mut EntityESPConfig) {
        nk.row_dynamic(30.0, 2)
            .label(title, TextAlignment::LEFT)
            .multi_select_combo(
                &["Boxes", "Names", "Health bar"],
                &mut [
                    &mut esp_cfg.boxes,
                    &mut esp_cfg.names,
                    &mut esp_cfg.health,
                ],
            );
    }

    nk.row_dynamic(30.0, 1)
        .checkbox("Master", &mut config.esp.master);

    if !config.esp.master {
        return;
    }

    nk.row_dynamic(30.0, 1)
        .label("Players", TextAlignment::LEFT)
        .horizontal_separator(1.0);

    entity_esp_combo(nk, "Enemy Players", &mut config.esp.player_enemy);
    entity_esp_combo(nk, "Friendly Players", &mut config.esp.player_friendly);

    nk.row_dynamic(30.0, 2)
        .label("Conditions", TextAlignment::LEFT)
        .multi_select_combo(
            &["Disguised", "Taunting", "Zoomed", "Invisible", "Milked", "No MG", "Butter"],
            &mut [
                &mut config.esp.conds.disguised.enabled,
                &mut config.esp.conds.taunting.enabled,
                &mut config.esp.conds.zoomed.enabled,
                &mut config.esp.conds.invisible.enabled,
                &mut config.esp.conds.milked.enabled,
                &mut config.esp.conds.mg.enabled,
                &mut config.esp.conds.butter.enabled,
            ],
        );

    nk.row_dynamic(30.0, 1)
        .label("Buildings", TextAlignment::LEFT)
        .horizontal_separator(1.0);

    entity_esp_combo(nk, "Enemy Buildings", &mut config.esp.building_enemy);
    entity_esp_combo(nk, "Friendly Buildings", &mut config.esp.building_friendly);

    nk.row_dynamic(30.0, 1)
        .label("Aimbot", TextAlignment::LEFT)
        .horizontal_separator(1.0)
        .row_dynamic(30.0, 1)
        .checkbox("Show target", &mut config.esp.aimbot_target);
}

fn colors_tab(nk: &Nuklear, config: &mut Config) {
    use crate::types::ColorF;

    fn pick(nk: &Nuklear, c: &mut ColorF) {
        nk.color_picker(&mut c.r, &mut c.g, &mut c.b, &mut c.a);
    }

    // --- Boxes ---
    if nk.tree_push("Boxes") {
        nk.row_dynamic(30.0, 1)
            .checkbox("Use team colors", &mut config.colors.boxes.use_team_colors);
        if !config.colors.boxes.use_team_colors {
            nk.row_dynamic(30.0, 2)
                .label("Enemy", TextAlignment::LEFT)
                .label("Friendly", TextAlignment::LEFT);
            nk.row_dynamic(200.0, 2);
            pick(nk, &mut config.colors.boxes.enemy);
            pick(nk, &mut config.colors.boxes.friendly);
        }
        nk.tree_pop();
    }

    // --- Names ---
    if nk.tree_push("Names") {
        nk.row_dynamic(30.0, 1)
            .checkbox("Use team colors", &mut config.colors.names.use_team_colors);
        if !config.colors.names.use_team_colors {
            nk.row_dynamic(30.0, 2)
                .label("Enemy", TextAlignment::LEFT)
                .label("Friendly", TextAlignment::LEFT);
            nk.row_dynamic(200.0, 2);
            pick(nk, &mut config.colors.names.enemy);
            pick(nk, &mut config.colors.names.friendly);
        }
        nk.tree_pop();
    }

    // --- Buildings ---
    if nk.tree_push("Buildings") {
        nk.row_dynamic(30.0, 1)
            .checkbox("Use team colors", &mut config.colors.buildings.use_team_colors);
        if !config.colors.buildings.use_team_colors {
            for (name, c) in [
                ("Sentry",     &mut config.colors.buildings.sentry),
                ("Dispenser",  &mut config.colors.buildings.dispenser),
                ("Teleporter", &mut config.colors.buildings.teleporter),
            ] {
                nk.row_dynamic(30.0, 1)
                    .label(name, TextAlignment::LEFT)
                    .horizontal_separator(1.0)
                    .row_dynamic(30.0, 2)
                    .label("Enemy", TextAlignment::LEFT)
                    .label("Friendly", TextAlignment::LEFT);
                nk.row_dynamic(200.0, 2);
                pick(nk, &mut c.enemy);
                pick(nk, &mut c.friendly);
            }
        }
        nk.tree_pop();
    }

    // --- Conditions ---
    let any_enabled = {
        let c = &config.esp.conds;
        c.disguised.enabled || c.taunting.enabled || c.zoomed.enabled
            || c.invisible.enabled || c.milked.enabled || c.mg.enabled || c.butter.enabled
    };

    if any_enabled && nk.tree_push("Conditions") {
        const COND_NAMES: [&str; 7] =
            ["Disguised", "Taunting", "Zoomed", "Invisible", "Milked", "No MG", "Butter"];

        let enabled_indices: Vec<usize> = {
            let c = &config.esp.conds;
            [c.disguised.enabled, c.taunting.enabled, c.zoomed.enabled,
             c.invisible.enabled, c.milked.enabled, c.mg.enabled, c.butter.enabled]
                .into_iter().enumerate().filter_map(|(i, e)| e.then_some(i)).collect()
        };

        let conds = &mut config.esp.conds;
        for chunk in enabled_indices.chunks(2) {
            let n = chunk.len() as i32;
            nk.row_dynamic(30.0, n);
            for &i in chunk {
                nk.label(COND_NAMES[i], TextAlignment::LEFT);
            }
            nk.row_dynamic(200.0, n);
            for &i in chunk {
                let c = match i {
                    0 => &mut conds.disguised.color,
                    1 => &mut conds.taunting.color,
                    2 => &mut conds.zoomed.color,
                    3 => &mut conds.invisible.color,
                    4 => &mut conds.milked.color,
                    5 => &mut conds.mg.color,
                    _ => &mut conds.butter.color,
                };
                pick(nk, c);
            }
        }
        nk.tree_pop();
    }

    // --- Player Categories (2x2 grid) ---
    if nk.tree_push("Player Categories") {
        let pc = &mut config.colors.player_categories;
        nk.row_dynamic(30.0, 2)
            .label("Category 1", TextAlignment::LEFT)
            .label("Category 2", TextAlignment::LEFT);
        nk.row_dynamic(200.0, 2);
        pick(nk, &mut pc.category1);
        pick(nk, &mut pc.category2);
        nk.row_dynamic(30.0, 2)
            .label("Category 3", TextAlignment::LEFT)
            .label("Category 4", TextAlignment::LEFT);
        nk.row_dynamic(200.0, 2);
        pick(nk, &mut pc.category3);
        pick(nk, &mut pc.category4);
        nk.tree_pop();
    }
}

fn misc_tab(nk: &Nuklear, config: &mut Config) {
    nk.row_dynamic(30.0, 1)
        .checkbox("Bunnyhop", &mut config.bunnyhop)
        .row_dynamic(30.0, 1)
        .checkbox("Spectator list", &mut config.spectator_list)
        .row_dynamic(30.0, 2)
        .checkbox("Thirdperson", &mut config.thirdperson.use_key);

    if config.thirdperson.use_key {
        let mut g = Globals::write();
        let label = if g.thirdperson_key_editing {
            "Press a key...".to_string()
        } else if config.thirdperson.is_mouse_button {
            format!("Thirdperson key: Mouse {}", config.thirdperson.code as u32)
        } else {
            format!("Thirdperson key: {}", config.thirdperson.code as u32)
        };

        if nk.button_label(label.as_str()) {
            g.thirdperson_key_editing = !g.thirdperson_key_editing;
        }
    }
}

fn config_tab(nk: &Nuklear, config: &mut Config) {
    {
        let configs = Config::list_configs();

        nk.row_dynamic(200.0, 1);
        if nk.group_begin("Configs", PanelFlags::BORDER) {
            for (i, cfg) in configs.iter().enumerate() {
                nk.layout_row_begin(LayoutFormat::DYNAMIC, 30.0, 2)
                    .layout_row_push(0.8);

                let mut selected = (unsafe { SELECTED_CONFIG } == i) as i32;
                if nk.selectable_label(cfg.as_str(), TextAlignment::LEFT, &mut selected) {
                    unsafe {
                        SELECTED_CONFIG = i;
                    }
                    config.load(cfg).unwrap(); // TODO: dont unwrap
                }

                nk.layout_row_push(0.2);

                if selected != 0 && nk.button_label("Save") {
                    config.save(cfg).unwrap(); // TODO: dont unwrap
                }

                nk.layout_row_end();
            }
            nk.group_end();
        }
    }

    nk.row_dynamic(30.0, 1);

    if nk.button_label("Refresh") {
        Config::refresh_configs();
    }

    let mut ptr = NEW_CONFIG_NAME.write().unwrap();
    nk.row_dynamic(30.0, 2)
        .edit_string(EditFlags::EDIT_FIELD, ptr.as_mut_ptr() as *mut i8, 256);
    if nk.button_label("Create new") {
        let end = ptr.iter().position(|&c| c == 0).unwrap_or(256);
        let name = str::from_utf8(&ptr[..end]).unwrap();
        config.save(name).unwrap(); // TODO: dont unwrap
        Config::refresh_configs();
    }
}

fn players_tab(nk: &Nuklear) {
    let engine = Interfaces::engine_client();

    if !engine.is_in_game() {
        nk.row_dynamic(30.0, 1).label("Not in a game.", TextAlignment::LEFT);
        return;
    }

    nk.layout_row_begin(LayoutFormat::DYNAMIC, 20.0, 3)
        .layout_row_push(0.30);
    nk.label("Name", TextAlignment::LEFT);
    nk.layout_row_push(0.45);
    nk.label("GUID", TextAlignment::LEFT);
    nk.layout_row_push(0.25);
    nk.label("Category", TextAlignment::LEFT);
    nk.layout_row_end();

    nk.row_dynamic(400.0, 1);
    if nk.group_begin("Players", PanelFlags::BORDER) {
        for i in 1..=engine.get_max_clients() {
            let info = engine.get_player_info(i);
            let name = str::from_utf8(&info.name).unwrap_or("").trim_end_matches('\0');
            if name.is_empty() {
                continue;
            }
            let guid = str::from_utf8(&info.guid).unwrap_or("").trim_end_matches('\0');
            let cat = player_db::get(guid);

            nk.layout_row_begin(LayoutFormat::DYNAMIC, 20.0, 3)
                .layout_row_push(0.30);
            nk.label(name, TextAlignment::LEFT);
            nk.layout_row_push(0.45);
            nk.label(guid, TextAlignment::LEFT);
            nk.layout_row_push(0.25);
            if nk.button_label(cat.name()) {
                player_db::set(guid, cat.next());
            }
            nk.layout_row_end();
        }
        nk.group_end();
    }
}
