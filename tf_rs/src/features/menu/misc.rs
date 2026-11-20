use nuklear::{Font, Nuklear};

use super::{ROW, field, key_button};
use crate::{config::Config, features::theme, globals::Globals};

pub(super) fn misc_tab(nk: &Nuklear, config: &mut Config) {
    nk.section("Movement", true)
        .row_dynamic(ROW, 1)
        .checkbox("Bunnyhop", &mut config.bunnyhop);

    nk.section("View", false)
        .row_dynamic(ROW, 1)
        .checkbox("Spectator list", &mut config.spectator_list)
        .checkbox("No zoom", &mut config.no_zoom)
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
