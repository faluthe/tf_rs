use std::sync::{Mutex, RwLock};

use nuklear::{
    Color, Font, Icon, Nuklear, Rect, Scroll,
    flags::{EditFlags, LayoutFormat, PanelFlags, TextAlignment},
};

use crate::{
    config::{Config, EntityESPConfig},
    features::theme,
    globals::Globals,
    interfaces::Interfaces,
    logger, player_db,
    types::Player,
};

// TODO: Clean this up lol
static mut TAB: MenuTab = MenuTab::Aimbot;
static mut SELECTED_CONFIG: usize = 0;
static NEW_CONFIG_NAME: RwLock<[u8; 256]> = RwLock::new([0; 256]);
static CAT_NAME_BUFS: [RwLock<[u8; 64]>; 4] = [
    RwLock::new([0u8; 64]),
    RwLock::new([0u8; 64]),
    RwLock::new([0u8; 64]),
    RwLock::new([0u8; 64]),
];
static CAT_NAMES_BUF_INIT: std::sync::Once = std::sync::Once::new();
/// Where the menu was last drawn. Nuklear forgets a window once it stops being drawn, so this
/// keeps the position and size across closing and reopening the menu.
static MENU_BOUNDS: Mutex<Rect> = Mutex::new(Rect {
    x: 200.0,
    y: 200.0,
    w: 900.0,
    h: 600.0,
});
static LOG_VIEW: Mutex<LogView> = Mutex::new(LogView {
    lines: Vec::new(),
    seen: 0,
    scroll: Scroll { x: 0, y: 0 },
});

struct LogView {
    lines: Vec<String>,
    /// Total line count when `lines` was copied from the logger
    seen: u64,
    scroll: Scroll,
}

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
                MenuTab::Aimbot => aimbot_tab(nk, &mut config),
                MenuTab::ESP => esp_tab(nk, &mut config),
                MenuTab::Colors => colors_tab(nk, &mut config),
                MenuTab::Misc => misc_tab(nk, &mut config),
                MenuTab::Config => config_tab(nk, &mut config),
                MenuTab::Players => players_tab(nk),
                MenuTab::Logging => logging_tab(nk),
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

fn aimbot_tab(nk: &Nuklear, config: &mut Config) {
    let aim = &mut config.aimbot;

    nk.section("Aimbot", true)
        .row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut aim.master);

    if !aim.master {
        return;
    }

    nk.row_dynamic(ROW, 1)
        .checkbox("Silent aim", &mut aim.silent_aim)
        .checkbox("Building aim", &mut aim.building_aim)
        .checkbox("Ignore invisible", &mut aim.ignore_invis);

    nk.row_dynamic(ROW, 2)
        .checkbox("Aim key", &mut aim.key.use_key);
    if aim.key.use_key {
        let mut g = Globals::write();
        key_button(
            nk,
            &mut g.aimbot_key_editing,
            aim.key.code,
            aim.key.is_mouse_button,
        );
    } else {
        nk.spacing(1);
    }

    nk.section("Targeting", false);
    field(nk, &format!("Field of view ({})", aim.fov), |nk| {
        nk.slider_int(1, &mut aim.fov, 100, 1);
    });
    nk.row_dynamic(ROW, 1)
        .checkbox("Draw FOV circle", &mut aim.draw_fov);
    field(nk, "Disable when spectated", |nk| {
        nk.single_select_combo(
            &["Off", "On", "Firstperson only"],
            &mut aim.disable_when_spectated,
        );
    });

    nk.section("Player categories", false);
    let cat_names = player_db::get_names();
    let items = [
        "None",
        cat_names[0].as_str(),
        cat_names[1].as_str(),
        cat_names[2].as_str(),
        cat_names[3].as_str(),
    ];
    let old_ignore = aim.ignore_category as usize;
    let old_priority = aim.priority_category as usize;
    let mut ignore_idx = old_ignore;
    let mut priority_idx = old_priority;
    field(nk, "Ignore", |nk| {
        nk.single_select_combo(&items, &mut ignore_idx);
    });
    field(nk, "Prioritize", |nk| {
        nk.single_select_combo(&items, &mut priority_idx);
    });
    if ignore_idx > 0 && ignore_idx == priority_idx {
        if ignore_idx != old_ignore {
            priority_idx = 0;
        } else {
            ignore_idx = 0;
        }
    }
    aim.ignore_category = ignore_idx as u8;
    aim.priority_category = priority_idx as u8;
}

fn esp_tab(nk: &Nuklear, config: &mut Config) {
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
}

fn colors_tab(nk: &Nuklear, config: &mut Config) {
    const PICKER: f32 = 180.0;

    fn pick(nk: &Nuklear, c: &mut Color) {
        nk.color_picker(c);
    }

    fn pair_labels(nk: &Nuklear, left: &str, right: &str) {
        nk.row_dynamic(ROW, 2)
            .dim_label(left, TextAlignment::LEFT)
            .dim_label(right, TextAlignment::LEFT);
    }

    nk.section("ESP colors", true);

    // --- Boxes ---
    if nk.tree_push("Boxes") {
        nk.row_dynamic(ROW, 1)
            .checkbox("Use team colors", &mut config.colors.boxes.use_team_colors);
        if !config.colors.boxes.use_team_colors {
            pair_labels(nk, "Enemy", "Friendly");
            nk.row_dynamic(PICKER, 2);
            pick(nk, &mut config.colors.boxes.enemy);
            pick(nk, &mut config.colors.boxes.friendly);
        }
        nk.tree_pop();
    }

    // --- Names ---
    if nk.tree_push("Names") {
        nk.row_dynamic(ROW, 1)
            .checkbox("Use team colors", &mut config.colors.names.use_team_colors);
        if !config.colors.names.use_team_colors {
            pair_labels(nk, "Enemy", "Friendly");
            nk.row_dynamic(PICKER, 2);
            pick(nk, &mut config.colors.names.enemy);
            pick(nk, &mut config.colors.names.friendly);
        }
        nk.tree_pop();
    }

    // --- Buildings ---
    if nk.tree_push("Buildings") {
        nk.row_dynamic(ROW, 1).checkbox(
            "Use team colors",
            &mut config.colors.buildings.use_team_colors,
        );
        if !config.colors.buildings.use_team_colors {
            for (name, c) in [
                ("Sentry", &mut config.colors.buildings.sentry),
                ("Dispenser", &mut config.colors.buildings.dispenser),
                ("Teleporter", &mut config.colors.buildings.teleporter),
            ] {
                pair_labels(
                    nk,
                    &format!("{name} / enemy"),
                    &format!("{name} / friendly"),
                );
                nk.row_dynamic(PICKER, 2);
                pick(nk, &mut c.enemy);
                pick(nk, &mut c.friendly);
            }
        }
        nk.tree_pop();
    }

    // --- Conditions ---
    let any_enabled = {
        let c = &config.esp.conds;
        c.disguised.enabled
            || c.taunting.enabled
            || c.zoomed.enabled
            || c.invisible.enabled
            || c.milked.enabled
            || c.mg.enabled
            || c.butter.enabled
            || c.category
    };

    if any_enabled && nk.tree_push("Conditions") {
        const COND_NAMES: [&str; 7] = [
            "Disguised",
            "Taunting",
            "Zoomed",
            "Invisible",
            "Milked",
            "No MG",
            "Butter",
        ];

        let enabled_indices: Vec<usize> = {
            let c = &config.esp.conds;
            [
                c.disguised.enabled,
                c.taunting.enabled,
                c.zoomed.enabled,
                c.invisible.enabled,
                c.milked.enabled,
                c.mg.enabled,
                c.butter.enabled,
            ]
            .into_iter()
            .enumerate()
            .filter_map(|(i, e)| e.then_some(i))
            .collect()
        };

        let conds = &mut config.esp.conds;
        for chunk in enabled_indices.chunks(2) {
            let n = chunk.len() as i32;
            nk.row_dynamic(ROW, n);
            for &i in chunk {
                nk.dim_label(COND_NAMES[i], TextAlignment::LEFT);
            }
            nk.row_dynamic(PICKER, n);
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
        let cat_names = player_db::get_names();
        pair_labels(nk, &cat_names[0], &cat_names[1]);
        nk.row_dynamic(PICKER, 2);
        pick(nk, &mut pc.category1);
        pick(nk, &mut pc.category2);
        pair_labels(nk, &cat_names[2], &cat_names[3]);
        nk.row_dynamic(PICKER, 2);
        pick(nk, &mut pc.category3);
        pick(nk, &mut pc.category4);
        nk.tree_pop();
    }
}

fn misc_tab(nk: &Nuklear, config: &mut Config) {
    nk.section("Movement", true)
        .row_dynamic(ROW, 1)
        .checkbox("Bunnyhop", &mut config.bunnyhop);

    nk.section("View", false)
        .row_dynamic(ROW, 1)
        .checkbox("Spectator list", &mut config.spectator_list)
        .row_dynamic(ROW, 2)
        .checkbox("Thirdperson key", &mut config.thirdperson.use_key);
    if config.thirdperson.use_key {
        let mut g = Globals::write();
        key_button(
            nk,
            &mut g.thirdperson_key_editing,
            config.thirdperson.code,
            config.thirdperson.is_mouse_button,
        );
    } else {
        nk.spacing(1);
    }

    nk.section("Interface", false);
    let theme_names = theme::THEMES.map(|t| t.name);
    field(nk, "Theme", |nk| {
        nk.single_select_combo(&theme_names, &mut config.menu.theme);
    });
    let font_names = Font::ALL.map(|f| f.name());
    field(nk, "Font", |nk| {
        nk.single_select_combo(&font_names, &mut config.menu.font);
    });

    nk.section("Session", false).row_dynamic(ROW, 1);
    if nk.button_label("Unload") {
        log::info!("Unload requested");
        Globals::write().unload_requested = true;
    }
}

fn config_tab(nk: &Nuklear, config: &mut Config) {
    {
        if nk.section_with_icon("Configurations", true, Icon::Refresh) {
            Config::refresh_configs();
        }
        let configs = Config::list_configs();

        nk.row_dynamic(220.0, 1);
        if nk.group_begin("Configs", PanelFlags::BORDER) {
            for (i, cfg) in configs.iter().enumerate() {
                nk.layout_row_begin(LayoutFormat::DYNAMIC, ROW, 2)
                    .layout_row_push(0.78);

                let mut selected = (unsafe { SELECTED_CONFIG } == i) as i32;
                if nk.selectable_label(cfg.as_str(), TextAlignment::LEFT, &mut selected) {
                    unsafe {
                        SELECTED_CONFIG = i;
                    }
                    if let Err(e) = config.load(cfg) {
                        log::error!("Failed to load config {cfg}: {e:#}");
                    }
                }

                nk.layout_row_push(0.22);

                if selected != 0 && nk.button_label("Save") {
                    if let Err(e) = config.save(cfg) {
                        log::error!("Failed to save config {cfg}: {e:#}");
                    }
                }

                nk.layout_row_end();
            }
            nk.group_end();
        }
    }

    nk.section("New configuration", false);
    let mut ptr = NEW_CONFIG_NAME.write().unwrap();
    nk.layout_row_begin(LayoutFormat::DYNAMIC, ROW, 2)
        .layout_row_push(0.67)
        .edit_string(EditFlags::EDIT_FIELD, ptr.as_mut_ptr() as *mut i8, 256)
        .layout_row_push(0.33);
    let create = nk.button_label("Create");
    nk.layout_row_end();
    if create {
        let end = ptr.iter().position(|&c| c == 0).unwrap_or(256);
        match str::from_utf8(&ptr[..end]) {
            Ok(name) if !name.is_empty() => {
                if let Err(e) = config.save(name) {
                    log::error!("Failed to create config {name}: {e:#}");
                }
                Config::refresh_configs();
            }
            Ok(_) => log::warn!("Config name is empty, not creating"),
            Err(e) => log::error!("Config name is not valid UTF-8: {e}"),
        }
    }
}

fn players_tab(nk: &Nuklear) {
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
    category_names(nk);
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
    nk.row_dynamic((height - 220.0).max(150.0), 1);
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

fn category_names(nk: &Nuklear) {
    nk.section("Category names", false).row_dynamic(ROW, 2);
    for i in 0..2 {
        let mut buf = CAT_NAME_BUFS[i].write().unwrap();
        nk.edit_string(EditFlags::EDIT_FIELD, buf.as_mut_ptr() as *mut i8, 64);
    }
    nk.row_dynamic(ROW, 2);
    for i in 2..4 {
        let mut buf = CAT_NAME_BUFS[i].write().unwrap();
        nk.edit_string(EditFlags::EDIT_FIELD, buf.as_mut_ptr() as *mut i8, 64);
    }
    nk.row_dynamic(ROW, 1);
    if nk.button_label("Save names") {
        for i in 0..4 {
            let buf = CAT_NAME_BUFS[i].read().unwrap();
            let end = buf.iter().position(|&c| c == 0).unwrap_or(64);
            let name = str::from_utf8(&buf[..end]).unwrap_or("");
            player_db::set_name(i, name);
        }
    }
}

fn logging_tab(nk: &Nuklear) {
    const ROW_HEIGHT: f32 = 18.0;
    // Row height plus the theme's vertical item spacing
    const ROW_STEP: u32 = ROW_HEIGHT as u32 + 5;

    let mut view = LOG_VIEW.lock().unwrap_or_else(|e| e.into_inner());
    let view = &mut *view;

    if let Some((lines, total)) = logger::lines_since(view.seen) {
        // Scroll past the new rows, nuklear clamps the offset to the bottom
        let new_rows = total.saturating_sub(view.seen).min(lines.len() as u64) as u32;
        view.scroll.y = view.scroll.y.saturating_add(new_rows * ROW_STEP);
        view.lines = lines;
        view.seen = total;
    }

    let text = nk.palette().map_or(Color::hex(0xc8c8c8), |p| p.text);

    // Fill the rest of the window
    nk.section("Log", true);
    let (_, height) = nk.get_content_region();
    nk.row_dynamic((height - 44.0).max(100.0), 1);
    nk.push_font(Font::Cousine);
    if nk.group_scrolled_begin("Log", PanelFlags::BORDER, &mut view.scroll) {
        nk.row_dynamic(ROW_HEIGHT, 1);
        for line in &view.lines {
            let (r, g, b) = if line.contains(" ERROR ") {
                (235, 80, 70)
            } else if line.contains(" WARN ") {
                (230, 180, 70)
            } else {
                (text.r, text.g, text.b)
            };
            nk.colored_label(line.as_str(), TextAlignment::LEFT, r, g, b, 255);
        }
        nk.group_end();
    }
    nk.pop_font();
}
