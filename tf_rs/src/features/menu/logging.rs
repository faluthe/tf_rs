use std::sync::Mutex;

use nuklear::{
    Color, Font, Nuklear, Scroll,
    flags::{PanelFlags, TextAlignment},
};

use crate::logger;

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

pub(super) fn logging_tab(nk: &Nuklear) {
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
