mod aimbot;
mod colors;
mod config;
mod esp;
mod logging;
mod misc;
mod players;

use std::sync::Mutex;

use nuklear::{
    Color, Nuklear, Rect,
    flags::{LayoutFormat, PanelFlags, TextAlignment},
};

use crate::config::Config;

// TODO: Clean this up lol
static mut TAB: MenuTab = MenuTab::Aimbot;
/// Where the menu was last drawn. Nuklear forgets a window once it stops being drawn, so this
/// keeps the position and size across closing and reopening the menu.
static MENU_BOUNDS: Mutex<Rect> = Mutex::new(Rect {
    x: 200.0,
    y: 200.0,
    w: 900.0,
    h: 600.0,
});

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuTab {
    Aimbot,
    ESP,
    Colors,
    Misc,
    Config,
    Players,
    Logging,
}

/// Height of a normal widget row
const ROW: f32 = 26.0;
/// Width of the navigation sidebar
const NAV_W: f32 = 128.0;
const TABS: [(&str, MenuTab); 7] = [
    ("Aimbot", MenuTab::Aimbot),
    ("ESP", MenuTab::ESP),
    ("Colors", MenuTab::Colors),
    ("Players", MenuTab::Players),
    ("Misc", MenuTab::Misc),
    ("Config", MenuTab::Config),
    ("Log", MenuTab::Logging),
];

pub fn draw(nk: &Nuklear) {
    let mut bounds = MENU_BOUNDS.lock().unwrap_or_else(|e| e.into_inner());
    if nk.begin(
        "TF_RS",
        PanelFlags::BORDER
            | PanelFlags::MOVABLE
            | PanelFlags::SCALABLE
            | PanelFlags::TITLE
            | PanelFlags::NO_SCROLLBAR,
        *bounds,
    ) {
        *bounds = nk.window_get_bounds();
        let mut config = Config::write();
        // Not theme::current(), that would read the config we are holding the write lock on
        let panel = nk.palette().map_or(Color::hex(0x121314), |p| p.panel);

        // Sidebar on the left, active tab on the right, both filling the window
        let (width, height) = nk.get_content_region();
        let height = (height - 4.0).max(100.0);
        // Minus the window's horizontal item spacing between the two groups
        let body_w = (width - NAV_W - 6.0).max(100.0);
        nk.layout_row_begin(LayoutFormat::STATIC, height, 2)
            .layout_row_push(NAV_W);

        nk.push_group_padding(0.0, 6.0);
        if nk.group_begin_bg("nav", PanelFlags::NO_SCROLLBAR, panel) {
            nav(nk);
            nk.group_end();
        }
        nk.pop_group_padding();

        nk.layout_row_push(body_w);
        if nk.group_begin("body", PanelFlags::empty()) {
            match unsafe { TAB } {
                MenuTab::Aimbot => aimbot::aimbot_tab(nk, &mut config),
                MenuTab::ESP => esp::esp_tab(nk, &mut config),
                MenuTab::Colors => colors::colors_tab(nk, &mut config),
                MenuTab::Misc => misc::misc_tab(nk, &mut config),
                MenuTab::Config => config::config_tab(nk, &mut config),
                MenuTab::Players => players::players_tab(nk),
                MenuTab::Logging => logging::logging_tab(nk),
            }
            nk.group_end();
        }
        nk.layout_row_end();
    }
    nk.end();
}

fn nav(nk: &Nuklear) {
    nk.row_dynamic(28.0, 1);
    for (title, tab) in TABS {
        if nk.nav_button(title, tab == unsafe { TAB }) {
            unsafe {
                TAB = tab;
            }
        }
    }
}

/// A row with a label on the left and one widget, drawn by `widget`, on the right
fn field(nk: &Nuklear, label: &str, widget: impl FnOnce(&Nuklear)) {
    nk.layout_row_begin(LayoutFormat::DYNAMIC, ROW, 2)
        .layout_row_push(0.45)
        .label(label, TextAlignment::LEFT)
        .layout_row_push(0.55);
    widget(nk);
    nk.layout_row_end();
}

/// Button that shows the bound key and starts listening for a new one when clicked
fn key_button(nk: &Nuklear, editing: &mut bool, code: u32, is_mouse_button: bool) {
    let label = if *editing {
        "Press a key...".to_string()
    } else if is_mouse_button {
        format!("Mouse {code}")
    } else {
        format!("Key {code}")
    };

    if nk.button_label(label.as_str()) {
        *editing = !*editing;
    }
}
