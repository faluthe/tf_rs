use std::ffi::c_void;

use crate::{features::esp, helpers, hooks::Hooks, interfaces::Interfaces};

pub extern "C" fn hk_paint_traverse(
    this: *mut c_void,
    panel: *mut c_void,
    force_repaint: i8,
    allow_force: i8,
) -> i64 {
    let rc = Hooks::paint_traverse()
        .original
        .call_paint_traverse(this, panel, force_repaint, allow_force)
        .expect("Invalid PaintTraverse function signature");

    if Interfaces::panel().panel_name(panel) != "FocusOverlayPanel" {
        return rc;
    }

    Interfaces::surface().draw_set_text_font(esp::esp_font());
    Interfaces::surface().draw_set_text_color(255, 255, 255, 255);

    if !Interfaces::engine_client().is_in_game() {
        return rc;
    }

    let localplayer = helpers::get_localplayer().expect("Failed to get localplayer");

    esp::player_esp(&localplayer);

    if localplayer.is_dead() {
        return rc;
    }

    esp::draw_fov();

    rc
}
