use std::ffi::c_void;

use crate::{hooks::Hooks, interfaces::Interfaces};

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

    Interfaces::surface().draw_set_color(255, 255, 255, 255);
    Interfaces::surface().draw_outlined_rect(10, 10, 100, 100);

    rc
}
