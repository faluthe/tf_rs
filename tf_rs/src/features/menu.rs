use nuklear::{
    Nuklear, Rect,
    flags::{PanelFlags, TextAlignment},
};

use crate::config::Config;

static mut TAB: MenuTab = MenuTab::Aimbot;

#[derive(Clone, Copy)]
enum MenuTab {
    Aimbot,
    ESP,
    Misc,
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

        nk.row_dynamic(30.0, 3);
        tab_button(nk, "Aimbot", MenuTab::Aimbot);
        tab_button(nk, "ESP", MenuTab::ESP);
        tab_button(nk, "Misc", MenuTab::Misc);

        match unsafe { TAB } {
            MenuTab::Aimbot => aimbot_tab(nk, &mut config),
            MenuTab::ESP => esp_tab(nk, &mut config),
            MenuTab::Misc => misc_tab(nk, &mut config),
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
