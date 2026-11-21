use nuklear::Nuklear;

use super::{ROW, field, key_button};
use crate::{config::Config, globals::Globals, player_db};

pub(super) fn aimbot_tab(nk: &Nuklear, config: &mut Config) {
    let aim = &mut config.aimbot;

    nk.section("Aimbot", true)
        .row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut aim.master);

    if aim.master {
        aimbot_settings(nk, config);
    }

    nk.section("Backtrack", false)
        .row_dynamic(ROW, 1)
        .checkbox("Enabled", &mut config.backtrack.enabled);
}

fn aimbot_settings(nk: &Nuklear, config: &mut Config) {
    let aim = &mut config.aimbot;

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
