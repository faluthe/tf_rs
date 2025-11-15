use nuklear::{
    Nuklear, Rect,
    flags::{PanelFlags, TextAlignment},
};

use crate::{cfg_enabled, cfg_get, config::CONFIG};

static mut TAB: MenuTab = MenuTab::Aimbot;

#[derive(Clone, Copy)]
enum MenuTab {
    Aimbot,
    ESP,
    Misc,
}

fn draw_tab(nk: &Nuklear, title: &str, tab: MenuTab) {
    if nk.button_label(title) {
        unsafe {
            TAB = tab;
        }
    }
}

pub fn draw(nk: &Nuklear) {
    if nk.begin(
        "TF_RS",
        PanelFlags::BORDER | PanelFlags::MOVABLE | PanelFlags::TITLE,
        Rect {
            x: 200.0,
            y: 200.0,
            w: 500.0,
            h: 400.0,
        },
    ) {
        nk.row_dynamic(30.0, 3);
        draw_tab(nk, "Aimbot", MenuTab::Aimbot);
        draw_tab(nk, "ESP", MenuTab::ESP);
        draw_tab(nk, "Misc", MenuTab::Misc);

        match unsafe { TAB } {
            MenuTab::Aimbot => {
                nk.row_dynamic(30.0, 1)
                    .checkbox("Master", CONFIG.aimbot.as_ptr());
                if cfg_enabled!(aimbot) {
                    nk.row_dynamic(30.0, 1)
                        .checkbox("Silent Aim", CONFIG.silent_aim.as_ptr())
                        .row_dynamic(30.0, 1)
                        .checkbox("Use key", CONFIG.use_aimbot_key.as_ptr())
                        .row_dynamic(30.0, 2)
                        .label(
                            format!("Aimbot FOV: {}", cfg_get!(aimbot_fov)),
                            TextAlignment::LEFT,
                        )
                        .slider_int(1, CONFIG.aimbot_fov.as_ptr(), 100, 1)
                        .row_dynamic(30.0, 1)
                        .checkbox("Draw fov", CONFIG.draw_fov.as_ptr());
                }
            }
            MenuTab::ESP => {
                nk.row_dynamic(30.0, 1)
                    .checkbox("ESP Boxes", CONFIG.esp.as_ptr());
            }
            MenuTab::Misc => {
                nk.row_dynamic(30.0, 1)
                    .checkbox("Bunnyhop", CONFIG.bunnyhop.as_ptr());
            }
        }
    }
    nk.end();
}
