use std::sync::Mutex;

use nuklear::{
    Color, Font, Nuklear, Scroll,
    flags::{PanelFlags, TextAlignment},
};

use log::{Level, LevelFilter};

use super::{ROW, field};
use crate::logger;

const LEVELS: [(&str, LevelFilter); 5] = [
    ("Error", LevelFilter::Error),
    ("Warn", LevelFilter::Warn),
    ("Info", LevelFilter::Info),
    ("Debug", LevelFilter::Debug),
    ("Trace", LevelFilter::Trace),
];

static LOG_VIEW: Mutex<LogView> = Mutex::new(LogView {
    lines: Vec::new(),
    seen: 0,
    scroll: Scroll { x: 0, y: 0 },
});

struct LogView {
    lines: Vec<(Level, String)>,
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
        let level = logger::level();
        let new_rows = total.saturating_sub(view.seen).min(lines.len() as u64) as usize;
        let new_rows = lines[lines.len() - new_rows..]
            .iter()
            .filter(|(l, _)| *l <= level)
            .count() as u32;
        view.scroll.y = view.scroll.y.saturating_add(new_rows * ROW_STEP);
        view.lines = lines;
        view.seen = total;
    }

    let text = nk.palette().map_or(Color::hex(0xc8c8c8), |p| p.text);

    nk.section("Log", true);
    let names = LEVELS.map(|(name, _)| name);
    let current = logger::level();
    let mut selected = LEVELS.iter().position(|&(_, l)| l == current).unwrap_or(2);
    field(nk, "Level", |nk| {
        nk.single_select_combo(&names, &mut selected);
    });
    if LEVELS[selected].1 != current {
        logger::set_level(LEVELS[selected].1);
        // Jump to the newest line, nuklear clamps the offset to the bottom
        view.scroll.y = u32::MAX;
    }

    // Fill the rest of the window, below the heading and level row
    let (_, height) = nk.get_content_region();
    nk.row_dynamic((height - 44.0 - ROW - 5.0).max(100.0), 1);
    nk.push_font(Font::Cousine);
    if nk.group_scrolled_begin("Log", PanelFlags::BORDER, &mut view.scroll) {
        nk.row_dynamic(ROW_HEIGHT, 1);
        let level = LEVELS[selected].1;
        for (line_level, line) in view.lines.iter().filter(|(l, _)| *l <= level) {
            let (r, g, b) = match line_level {
                Level::Error => (235, 80, 70),
                Level::Warn => (230, 180, 70),
                _ => (text.r, text.g, text.b),
            };
            nk.colored_label(line.as_str(), TextAlignment::LEFT, r, g, b, 255);
        }
        nk.group_end();
    }
    nk.pop_font();
}
