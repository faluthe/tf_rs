use std::sync::RwLock;

use nuklear::{
    Nuklear, Rect,
    flags::{EditFlags, LayoutFormat, PanelFlags, TextAlignment},
};

use crate::{
    config::{Config, EntityESPConfig},
    globals::Globals,
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
    Misc,
    Config,
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

        nk.row_dynamic(30.0, 4);
        tab_button(nk, "Aimbot", MenuTab::Aimbot);
        tab_button(nk, "ESP", MenuTab::ESP);
        tab_button(nk, "Misc", MenuTab::Misc);
        tab_button(nk, "Config", MenuTab::Config);

        match unsafe { TAB } {
            MenuTab::Aimbot => aimbot_tab(nk, &mut config),
            MenuTab::ESP => esp_tab(nk, &mut config),
            MenuTab::Misc => misc_tab(nk, &mut config),
            MenuTab::Config => config_tab(nk, &mut config),
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

    nk.set_button_normal_color(color.r as u8, color.g as u8, color.b as u8, color.a as u8);

    if nk.button_label(title) {
        unsafe {
            TAB = tab;
        }
    }

    // Restore default color
    nk.set_button_normal_color(
        rgba::LIGHT_GREY.r as u8,
        rgba::LIGHT_GREY.g as u8,
        rgba::LIGHT_GREY.b as u8,
        rgba::LIGHT_GREY.a as u8,
    );
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
                &["Boxes", "Names", "Health bar", "Conditions"],
                &mut [
                    &mut esp_cfg.boxes,
                    &mut esp_cfg.names,
                    &mut esp_cfg.health,
                    &mut esp_cfg.conds,
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
