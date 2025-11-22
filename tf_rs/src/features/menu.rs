use nuklear::{
    Nuklear, Rect,
    flags::{LayoutFormat, PanelFlags, TextAlignment},
};

use crate::config::Config;

static mut TAB: MenuTab = MenuTab::Aimbot;
static mut SELECTED_CONFIG: usize = 0;

#[derive(Clone, Copy)]
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
            w: 600.0,
            h: 400.0,
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
    if nk.button_label(title) {
        unsafe {
            TAB = tab;
        }
    }
}

fn aimbot_tab(nk: &Nuklear, config: &mut Config) {
    nk.row_dynamic(30.0, 1)
        .checkbox("Master", &mut config.aimbot.master);

    if config.aimbot.master != 0 {
        nk.row_dynamic(30.0, 1)
            .checkbox("Silent aim", &mut config.aimbot.silent_aim)
            .row_dynamic(30.0, 1)
            .checkbox("Use key", &mut config.aimbot.use_key)
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
    nk.row_dynamic(30.0, 1)
        .checkbox("Master", &mut config.esp.master);

    if config.esp.master != 0 {
        nk.row_dynamic(30.0, 3)
            .checkbox("Boxes", &mut config.esp.boxes)
            .checkbox("Names", &mut config.esp.names)
            .checkbox("Aimbot target", &mut config.esp.aimbot_target);
    }
}

fn misc_tab(nk: &Nuklear, config: &mut Config) {
    nk.row_dynamic(30.0, 1)
        .checkbox("Bunnyhop", &mut config.bunnyhop);
}

fn config_tab(nk: &Nuklear, config: &mut Config) {
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

    nk.row_dynamic(30.0, 2)
        .label("some config", TextAlignment::LEFT);
    if nk.button_label("Create new") {
        // create new config
    }
}
