use nuklear::{Color, Nuklear, flags::TextAlignment};

use super::ROW;
use crate::{config::Config, player_db};

pub(super) fn colors_tab(nk: &Nuklear, config: &mut Config) {
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
