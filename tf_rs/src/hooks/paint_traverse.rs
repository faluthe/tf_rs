use std::ffi::c_void;

use crate::{
    config::Config,
    features::{esp, preview},
    helpers,
    hooks::Hooks,
    interfaces::Interfaces,
};

pub extern "C" fn hk_paint_traverse(
    this: *mut c_void,
    panel: *mut c_void,
    force_repaint: i8,
    allow_force: i8,
) -> i64 {
    let panel_iface = Interfaces::panel();
    let panel_name = panel_iface.panel_name(panel);

    if panel_name == "HudScope" && Config::read().remove_scope {
        return 0;
    }

    let rc = Hooks::paint_traverse()
        .call_paint_traverse(this, panel, force_repaint, allow_force)
        .expect("Invalid PaintTraverse function signature");

    if panel_name != "FocusOverlayPanel" {
        return rc;
    }

    let surface = Interfaces::surface();

    surface.draw_set_text_font(esp::esp_font(&surface));

    let localplayer = Interfaces::engine_client()
        .is_in_game()
        .then(helpers::get_localplayer)
        .flatten();
    // A copy, holding the lock while drawing can deadlock with the menu on the render thread
    let config = Config::read().clone();

    if let Some(localplayer) = &localplayer {
        esp::run(localplayer, &surface, &config);

        if !localplayer.is_dead() {
            esp::draw_fov(&surface, &config);
            esp::draw_prediction(&surface, &config);
        }
    }

    // Last so nothing drawn here covers it, and outside a map too
    preview::draw(localplayer.as_ref(), &surface, &config);

    rc
}
