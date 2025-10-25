use bitflags::bitflags;

use nuklear_sys::{
    nk_panel_flags_NK_WINDOW_BORDER, nk_panel_flags_NK_WINDOW_MOVABLE,
    nk_panel_flags_NK_WINDOW_TITLE, nk_text_alignment_NK_TEXT_LEFT,
};

bitflags! {
    pub struct PanelFlags : u32{
        const BORDER = nk_panel_flags_NK_WINDOW_BORDER;
        const MOVABLE = nk_panel_flags_NK_WINDOW_MOVABLE;
        const TITLE = nk_panel_flags_NK_WINDOW_TITLE;
    }
}

bitflags! {
    pub struct TextAlignment : u32 {
        const LEFT = nk_text_alignment_NK_TEXT_LEFT;
    }
}
