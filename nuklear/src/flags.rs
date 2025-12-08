use bitflags::bitflags;

use nuklear_sys::{
    nk_edit_types_NK_EDIT_FIELD, nk_layout_format_NK_DYNAMIC, nk_layout_format_NK_STATIC,
    nk_panel_flags_NK_WINDOW_BORDER, nk_panel_flags_NK_WINDOW_MOVABLE,
    nk_panel_flags_NK_WINDOW_NO_SCROLLBAR, nk_panel_flags_NK_WINDOW_TITLE,
    nk_text_alignment_NK_TEXT_CENTERED, nk_text_alignment_NK_TEXT_LEFT,
};

bitflags! {
    pub struct PanelFlags : u32{
        const BORDER = nk_panel_flags_NK_WINDOW_BORDER;
        const MOVABLE = nk_panel_flags_NK_WINDOW_MOVABLE;
        const TITLE = nk_panel_flags_NK_WINDOW_TITLE;
        const NO_SCROLLBAR = nk_panel_flags_NK_WINDOW_NO_SCROLLBAR;
    }
}

bitflags! {
    pub struct EditFlags : u32 {
        const EDIT_FIELD = nk_edit_types_NK_EDIT_FIELD;
    }
}

#[repr(u32)]
pub enum TextAlignment {
    LEFT = nk_text_alignment_NK_TEXT_LEFT,
    CENTER = nk_text_alignment_NK_TEXT_CENTERED,
}

#[repr(u32)]
pub enum LayoutFormat {
    DYNAMIC = nk_layout_format_NK_DYNAMIC,
    STATIC = nk_layout_format_NK_STATIC,
}
