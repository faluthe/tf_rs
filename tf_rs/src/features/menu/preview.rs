use nuklear::{Color, Nuklear, Rect, flags::PanelFlags};

use super::ROW;
use crate::features::preview::{self, CLASSES, Placement, Subject};

const TITLE: &str = "Preview";
/// Width of the panel next to the menu
const PREVIEW_W: f32 = 280.0;
/// Space between the menu and the panel
const GAP: f32 = 8.0;
/// Height of the tab bar and the controls row with the spacing after each
const CONTROLS_H: f32 = 28.0 + ROW + 12.0;

/// See-through panel against the right of the menu at `menu`, the game draws the preview in it
pub(super) fn preview_panel(nk: &Nuklear, menu: Rect) {
    let rect = Rect {
        x: menu.x + menu.w + GAP,
        y: menu.y,
        w: PREVIEW_W,
        h: menu.h,
    };
    // Follows the menu instead of being moved on its own
    nk.window_set_bounds(TITLE, rect);

    let mut placement = None;
    let clear = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    if nk.begin_bg(
        TITLE,
        PanelFlags::BORDER | PanelFlags::NO_SCROLLBAR,
        rect,
        clear,
    ) {
        let labels = Subject::ALL.map(Subject::label);
        let mut selected = preview::subject() as usize;
        nk.tab_bar(&labels, &mut selected);
        preview::set_subject(selected);

        let mut occluded = preview::occluded();
        if preview::shows_class_model() {
            let mut class = preview::class();
            nk.row_dynamic(ROW, 2)
                .checkbox("Occluded", &mut occluded)
                .single_select_combo(&CLASSES, &mut class);
            preview::set_class(class);
        } else {
            nk.row_dynamic(ROW, 1).checkbox("Occluded", &mut occluded);
        }
        preview::set_occluded(occluded);

        let (_, height) = nk.get_content_region();
        nk.row_dynamic((height - CONTROLS_H).max(32.0), 1);
        let model = nk.reserve_slot();
        if let Some(dx) = nk.drag_x(model) {
            preview::rotate(dx);
        }
        placement = Some(Placement {
            panel: nk.window_get_bounds(),
            model,
        });
    }
    nk.end();
    preview::set_placement(placement);
}
