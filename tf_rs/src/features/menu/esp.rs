use std::sync::atomic::{AtomicUsize, Ordering};

use nuklear::{Color, Nuklear, Swatch, ToggleColor};

use super::{ROW, field};
use crate::{
    config::{
        BuildingsColorsConfig, ChamsConfig, ChamsGroupConfig, ChamsLayerConfig, ChamsTargetConfig,
        ColorsConfig, CondsDisplayConfig, Config, EntityESPConfig, EspColorConfig, GlowConfig,
        GlowGroupConfig,
    },
    features::chams,
    player_db,
};

/// Width of the popup an ESP row opens
const ESP_POPUP_W: f32 = 280.0;

pub(super) fn esp_tab(nk: &Nuklear, config: &mut Config) {
    let Config { esp, colors, .. } = config;

    nk.section("ESP", true)
        .row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut esp.master);

    if !esp.master {
        return;
    }

    nk.section("Players", false);
    player_esp(nk, "Enemies", &mut esp.player_enemy, colors, false);
    player_esp(nk, "Teammates", &mut esp.player_friendly, colors, true);
    field(nk, "Team colors", |nk| {
        nk.multi_select_combo(
            &["Boxes", "Names"],
            &mut [
                &mut colors.boxes.use_team_colors,
                &mut colors.names.use_team_colors,
            ],
        );
    });
    conditions(nk, &mut esp.conds);

    nk.section("Buildings", false);
    building_esp(
        nk,
        "Enemy",
        &mut esp.building_enemy,
        &mut colors.buildings,
        false,
    );
    building_esp(
        nk,
        "Friendly",
        &mut esp.building_friendly,
        &mut colors.buildings,
        true,
    );
    nk.row_dynamic(ROW, 1)
        .checkbox("Team colors", &mut colors.buildings.use_team_colors);

    nk.section("Other", false)
        .row_dynamic(ROW, 1)
        .checkbox("Show aimbot target", &mut esp.aimbot_target)
        .checkbox("Ignore invisible", &mut esp.ignore_invis);

    chams_section(nk, &mut esp.chams);
    glow_section(nk, &mut esp.glow);
}

/// Names of the parts of `cfg` that are drawn
fn esp_parts(cfg: &EntityESPConfig) -> Vec<&'static str> {
    [
        (cfg.boxes, "Boxes"),
        (cfg.names, "Names"),
        (cfg.health, "Health bar"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some(name))
    .collect()
}

/// The configured color for one team, or none while team colors are used
fn custom_color(c: &mut EspColorConfig, friendly: bool) -> Option<&mut Color> {
    match (c.use_team_colors, friendly) {
        (true, _) => None,
        (false, true) => Some(&mut c.friendly),
        (false, false) => Some(&mut c.enemy),
    }
}

fn player_esp(
    nk: &Nuklear,
    title: &str,
    cfg: &mut EntityESPConfig,
    colors: &mut ColorsConfig,
    friendly: bool,
) {
    let names = esp_parts(cfg);
    let ColorsConfig {
        boxes,
        names: name_colors,
        ..
    } = colors;
    let swatches: Vec<Swatch> = [(cfg.boxes, &mut *boxes), (cfg.names, &mut *name_colors)]
        .into_iter()
        .filter(|(on, _)| *on)
        .filter_map(|(_, c)| custom_color(c, friendly).map(|c| Swatch::On(*c)))
        .collect();

    field(nk, title, |nk| {
        nk.summary_combo(&names, &swatches, ESP_POPUP_W, |nk| {
            nk.toggle_color_list(&mut [
                ToggleColor::new("Boxes", &mut cfg.boxes, custom_color(boxes, friendly)),
                ToggleColor::new("Names", &mut cfg.names, custom_color(name_colors, friendly)),
                ToggleColor::new("Health bar", &mut cfg.health, None),
            ]);
        });
    });
}

fn building_esp(
    nk: &Nuklear,
    title: &str,
    cfg: &mut EntityESPConfig,
    colors: &mut BuildingsColorsConfig,
    friendly: bool,
) {
    let names = esp_parts(cfg);
    let custom = !colors.use_team_colors && (cfg.boxes || cfg.names);
    let BuildingsColorsConfig {
        sentry,
        dispenser,
        teleporter,
        ..
    } = colors;
    let mut team_colors = [
        ("Sentry", sentry),
        ("Dispenser", dispenser),
        ("Teleporter", teleporter),
    ]
    .map(|(name, c)| {
        let color = if friendly {
            &mut c.friendly
        } else {
            &mut c.enemy
        };
        (name, color)
    });
    let swatches: Vec<Swatch> = if custom {
        team_colors.iter().map(|(_, c)| Swatch::On(**c)).collect()
    } else {
        Vec::new()
    };

    field(nk, title, |nk| {
        nk.summary_combo(&names, &swatches, ESP_POPUP_W, |nk| {
            let mut items = vec![
                ToggleColor::new("Boxes", &mut cfg.boxes, None),
                ToggleColor::new("Names", &mut cfg.names, None),
                ToggleColor::new("Health bar", &mut cfg.health, None),
            ];
            if !colors.use_team_colors {
                items.extend(
                    team_colors
                        .iter_mut()
                        .map(|(name, c)| ToggleColor::color(name, c)),
                );
            }
            nk.toggle_color_list(&mut items);
        });
    });
}

fn conditions(nk: &Nuklear, conds: &mut CondsDisplayConfig) {
    let CondsDisplayConfig {
        disguised,
        taunting,
        zoomed,
        invisible,
        milked,
        mg,
        butter,
        category,
    } = conds;
    let mut list = [
        ("Disguised", disguised),
        ("Taunting", taunting),
        ("Zoomed", zoomed),
        ("Invisible", invisible),
        ("Milked", milked),
        ("No MG", mg),
        ("Butter", butter),
    ];

    let mut names: Vec<&str> = list
        .iter()
        .filter_map(|(name, c)| c.enabled.then_some(*name))
        .collect();
    if *category {
        names.push("Category");
    }
    let swatches: Vec<Swatch> = list
        .iter()
        .filter(|(_, c)| c.enabled)
        .map(|(_, c)| Swatch::On(c.color))
        .collect();

    field(nk, "Conditions", |nk| {
        nk.summary_combo(&names, &swatches, ESP_POPUP_W, |nk| {
            let mut items: Vec<ToggleColor> = list
                .iter_mut()
                .map(|(name, c)| ToggleColor::new(name, &mut c.enabled, Some(&mut c.color)))
                .collect();
            items.push(ToggleColor::new("Category", category, None));
            nk.toggle_color_list(&mut items);
        });
    });
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
        1 => {
            chams_group(nk, "Enemy", &mut chams.building_enemy);
            chams_group(nk, "Friendly", &mut chams.building_friendly);
        }
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
            target_group(nk, "Aimbot target", &mut chams.aimbot_target);
            chams_group(nk, "Teammates", &mut chams.friendly);
            chams_group(nk, "Local player", &mut chams.local);
            chams_group(nk, "Backtrack", &mut chams.backtrack);
            chams_group(nk, "Backtrack target", &mut chams.backtrack_target);
        }
    }
}

/// Width of the popup a chams row opens
const CHAMS_POPUP_W: f32 = 300.0;

/// Layer being edited in whichever chams popup is open
static CHAMS_LAYER: AtomicUsize = AtomicUsize::new(0);
const LAYER_TABS: [&str; ChamsGroupConfig::LAYERS] = ["Base", "Overlay 1", "Overlay 2"];

fn chams_group(nk: &Nuklear, title: &str, group: &mut ChamsGroupConfig) {
    layered_group(nk, title, group, true);
}

// Viewmodels are never behind anything, so only the visible pass applies
fn first_person_group(nk: &Nuklear, title: &str, group: &mut ChamsGroupConfig) {
    layered_group(nk, title, group, false);
}

/// Row with a combo showing the materials of the layers in use and their colors, the popup
/// picks a layer, then its material and passes
fn layered_group(nk: &Nuklear, title: &str, group: &mut ChamsGroupConfig, occluded: bool) {
    let layers = group.layers();
    let mut shown: Vec<&ChamsLayerConfig> = layers
        .iter()
        .filter(|l| l.visible || (occluded && l.occluded))
        .collect();
    if shown.is_empty() {
        shown.push(&layers[0]);
    }
    let names: Vec<&str> = shown.iter().map(|l| material_label(l.material)).collect();
    let swatches: Vec<Swatch> = shown
        .iter()
        .filter(|l| !chams::is_original(l.material))
        .flat_map(|l| {
            let mut s = vec![Swatch::new(l.visible, l.visible_color)];
            if occluded {
                s.push(Swatch::new(l.occluded, l.occluded_color));
            }
            s
        })
        .collect();

    field(nk, title, |nk| {
        nk.summary_combo(&names, &swatches, CHAMS_POPUP_W, |nk| {
            let mut index = CHAMS_LAYER.load(Ordering::Relaxed);
            nk.tab_bar(&LAYER_TABS, &mut index);
            CHAMS_LAYER.store(index, Ordering::Relaxed);

            let layer = group.layer_mut(index);
            material_grid(nk, layer.material);
            if chams::is_original(*layer.material) {
                // Drawn as is, there's nothing to color and it can't show through walls
                nk.toggle_color_list(&mut [ToggleColor::new("Enabled", layer.visible, None)]);
            } else if occluded {
                nk.toggle_color_list(&mut [
                    ToggleColor::new("Visible", layer.visible, Some(layer.visible_color)),
                    ToggleColor::new("Occluded", layer.occluded, Some(layer.occluded_color)),
                ]);
            } else {
                nk.toggle_color_list(&mut [ToggleColor::new(
                    "Enabled",
                    layer.visible,
                    Some(layer.visible_color),
                )]);
            }
        });
    });
}

/// Colors the aimbot target is drawn in, with the materials of the group it's in
fn target_group(nk: &Nuklear, title: &str, target: &mut ChamsTargetConfig) {
    let ChamsTargetConfig {
        visible,
        occluded,
        visible_color,
        occluded_color,
    } = target;
    passes(nk, title, visible, occluded, visible_color, occluded_color);
}

/// Row with a combo saying whether either pass is on, the popup toggles and colors them
fn passes(
    nk: &Nuklear,
    title: &str,
    visible: &mut bool,
    occluded: &mut bool,
    visible_color: &mut Color,
    occluded_color: &mut Color,
) {
    let swatches = [
        Swatch::new(*visible, *visible_color),
        Swatch::new(*occluded, *occluded_color),
    ];
    let name = if *visible || *occluded { "On" } else { "Off" };
    field(nk, title, |nk| {
        nk.summary_combo(&[name], &swatches, CHAMS_POPUP_W, |nk| {
            nk.toggle_color_list(&mut [
                ToggleColor::new("Visible", visible, Some(visible_color)),
                ToggleColor::new("Occluded", occluded, Some(occluded_color)),
            ]);
        });
    });
}

/// Glow tab shown in the menu, index into `GLOW_TABS`
static GLOW_TAB: AtomicUsize = AtomicUsize::new(0);
const GLOW_TABS: [&str; 3] = ["Players", "Buildings", "Categories"];

fn glow_section(nk: &Nuklear, glow: &mut GlowConfig) {
    nk.section("Glow", false)
        .row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut glow.enabled);
    if !glow.enabled {
        return;
    }

    let mut tab = GLOW_TAB.load(Ordering::Relaxed);
    nk.tab_bar(&GLOW_TABS, &mut tab);
    GLOW_TAB.store(tab, Ordering::Relaxed);

    match tab {
        1 => {
            glow_group(nk, "Enemy", &mut glow.building_enemy);
            glow_group(nk, "Friendly", &mut glow.building_friendly);
        }
        2 => {
            let names = player_db::get_names();
            glow_group(nk, &names[0], &mut glow.category1);
            glow_group(nk, &names[1], &mut glow.category2);
            glow_group(nk, &names[2], &mut glow.category3);
            glow_group(nk, &names[3], &mut glow.category4);
        }
        _ => {
            glow_group(nk, "Enemies", &mut glow.enemy);
            glow_group(nk, "Aimbot target", &mut glow.aimbot_target);
            glow_group(nk, "Teammates", &mut glow.friendly);
            glow_group(nk, "Local player", &mut glow.local);
        }
    }
}

fn glow_group(nk: &Nuklear, title: &str, group: &mut GlowGroupConfig) {
    let GlowGroupConfig {
        visible,
        occluded,
        visible_color,
        occluded_color,
    } = group;
    passes(nk, title, visible, occluded, visible_color, occluded_color);
}

fn material_label(material: usize) -> &'static str {
    chams::MATERIALS.get(material).map_or("None", |m| m.label)
}

fn material_grid(nk: &Nuklear, material: &mut usize) {
    let labels: Vec<&str> = chams::MATERIALS.iter().map(|m| m.label).collect();
    nk.select_grid(&labels, material, 3);
}
