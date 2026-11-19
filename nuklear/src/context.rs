use std::{ffi::CString, sync::OnceLock};

use nuklear_sys::{
    GLEW_OK, SDL_Event, SDL_GL_CreateContext, SDL_GL_GetCurrentContext, SDL_GLContext, SDL_Window,
    glewInit, nk_anti_aliasing_NK_ANTI_ALIASING_ON, nk_begin, nk_bool, nk_button_label,
    nk_checkbox_label, nk_color, nk_color_format_NK_RGBA, nk_color_picker, nk_colorf,
    nk_collapse_states_NK_MINIMIZED, nk_combo_begin_label, nk_combo_end, nk_context,
    nk_edit_string_zero_terminated, nk_end, nk_filter_default, nk_flags, nk_font_atlas,
    nk_group_begin, nk_group_end, nk_group_scrolled_begin, nk_input_begin, nk_input_end, nk_input_is_key_released,
    nk_label, nk_label_colored, nk_layout_row_begin, nk_layout_row_dynamic, nk_layout_row_end,
    nk_layout_row_push, nk_rect, nk_rule_horizontal, nk_scroll, nk_sdl_font_stash_begin,
    nk_sdl_font_stash_end, nk_sdl_handle_event, nk_sdl_init, nk_sdl_render, nk_selectable_label,
    nk_slider_int, nk_tree_pop, nk_tree_push_hashed, nk_tree_type_NK_TREE_NODE, nk_vec2,
    nk_widget_width, nk_window_get_bounds, nk_window_get_content_region, nk_window_set_bounds,
    nk_font, nk_font_atlas_add_from_memory, nk_style_pop_font, nk_style_push_font,
    nk_style_set_font, nk_fill_rect, nk_spacer, nk_style_item_color, nk_widget_bounds,
    nk_window_get_canvas, nk_text_alignment_NK_TEXT_LEFT,
    nk_style_push_vec2, nk_style_pop_vec2,
};

use crate::style::{Color, FONT_HEADING, Font, HEADING_SIZE, Palette};

static CONTEXT: OnceLock<Context> = OnceLock::new();

#[derive(Clone, Copy)]
pub(crate) struct Context {
    pub nk_ctx: *mut nk_context,
    pub og_ctx: SDL_GLContext,
    pub new_ctx: SDL_GLContext,
    /// Indexed like `Font::ALL`
    fonts: [*mut nk_font; Font::ALL.len()],
    heading_font: *mut nk_font,
}

// TODO: Is this actually thread safe? Pottentially not, trace poll_event and swap_window usage
unsafe impl Sync for Context {}
unsafe impl Send for Context {}

impl Context {
    pub(crate) fn get_or_init(window: *mut SDL_Window) -> &'static Context {
        CONTEXT.get_or_init(|| unsafe {
            let og_ctx = SDL_GL_GetCurrentContext();
            let new_ctx = SDL_GL_CreateContext(window);

            let err = glewInit();
            if err != GLEW_OK {
                panic!("Failed to initialize GLEW");
            }

            let nk_ctx = nk_sdl_init(window);

            let mut atlas: *mut nk_font_atlas = std::ptr::null_mut();
            nk_sdl_font_stash_begin(&mut atlas);
            // The atlas only reads the TTF data while baking in stash_end, and the slices are
            // static, so handing out a mut pointer to them is fine
            let add = |data: &'static [u8], size: f32| {
                nk_font_atlas_add_from_memory(
                    atlas,
                    data.as_ptr() as *mut _,
                    data.len() as _,
                    size,
                    std::ptr::null(),
                )
            };
            // First font added becomes the default
            let fonts = Font::ALL.map(|f| {
                let (data, size) = f.data();
                add(data, size)
            });
            let heading_font = add(FONT_HEADING, HEADING_SIZE);
            nk_sdl_font_stash_end();

            // Open the first input window so poll_event calls have a valid begin/end block
            nk_input_begin(nk_ctx);

            Context {
                nk_ctx,
                og_ctx,
                new_ctx,
                fonts,
                heading_font,
            }
        })
    }

    pub(crate) fn row_dynamic(&self, height: f32, cols: i32) {
        unsafe {
            nk_layout_row_dynamic(self.nk_ctx, height, cols);
        }
    }

    pub(crate) fn label(&self, text: CString, alignment: u32) {
        unsafe {
            nk_label(self.nk_ctx, text.as_ptr(), alignment);
        }
    }

    pub(crate) fn colored_label(&self, text: CString, alignment: u32, r: u8, g: u8, b: u8, a: u8) {
        unsafe {
            let color = nk_color { r, g, b, a };
            nk_label_colored(self.nk_ctx, text.as_ptr(), alignment, color);
        }
    }

    pub(crate) fn checkbox_label(&self, text: CString, active: *mut i32) {
        unsafe {
            nk_checkbox_label(self.nk_ctx, text.as_ptr(), active);
        }
    }

    pub(crate) fn button_label(&self, label: CString) -> bool {
        unsafe { nk_button_label(self.nk_ctx, label.as_ptr()) != 0 }
    }

    pub(crate) fn slider_int(&self, min: i32, val: *mut i32, max: i32, step: i32) {
        unsafe {
            nk_slider_int(self.nk_ctx, min, val, max, step);
        }
    }

    pub(crate) fn rule_horizontal(&self, r: u8, g: u8, b: u8, a: u8, rounding: i32) {
        unsafe {
            let color = nk_color { r, g, b, a };
            nk_rule_horizontal(self.nk_ctx, color, rounding);
        }
    }

    pub(crate) fn window_get_bounds(&self) -> nk_rect {
        unsafe { nk_window_get_bounds(self.nk_ctx) }
    }

    pub(crate) fn window_get_content_region(&self) -> (f32, f32) {
        unsafe {
            let rect = nk_window_get_content_region(self.nk_ctx);
            (rect.w, rect.h)
        }
    }

    pub(crate) fn group_begin(&self, title: CString, flags: nk_flags) -> bool {
        unsafe { nk_group_begin(self.nk_ctx, title.as_ptr(), flags) != 0 }
    }

    pub(crate) fn group_scrolled_begin(
        &self,
        title: CString,
        scroll: *mut nk_scroll,
        flags: nk_flags,
    ) -> bool {
        unsafe { nk_group_scrolled_begin(self.nk_ctx, scroll, title.as_ptr(), flags) != 0 }
    }

    pub(crate) fn layout_row_begin(&self, fmt: u32, row_height: f32, cols: i32) {
        unsafe {
            nk_layout_row_begin(self.nk_ctx, fmt, row_height, cols);
        }
    }

    pub(crate) fn layout_row_push(&self, width: f32) {
        unsafe {
            nk_layout_row_push(self.nk_ctx, width);
        }
    }

    pub(crate) fn layout_row_end(&self) {
        unsafe {
            nk_layout_row_end(self.nk_ctx);
        }
    }

    pub(crate) fn group_end(&self) {
        unsafe {
            nk_group_end(self.nk_ctx);
        }
    }

    pub(crate) fn selectable_label(&self, label: CString, align: u32, selected: *mut i32) -> bool {
        unsafe { nk_selectable_label(self.nk_ctx, label.as_ptr(), align, selected) != 0 }
    }

    pub(crate) fn edit_string_zero_terminated(&self, flags: u32, buffer: *mut i8, max: i32) {
        unsafe {
            nk_edit_string_zero_terminated(
                self.nk_ctx,
                flags,
                buffer,
                max,
                Some(nk_filter_default),
            );
        }
    }

    pub(crate) fn input_begin(&self) {
        unsafe {
            nk_input_begin(self.nk_ctx);
        }
    }

    pub(crate) fn input_end(&self) {
        unsafe {
            nk_input_end(self.nk_ctx);
        }
    }

    pub(crate) fn handle_event(event: *mut SDL_Event) -> nk_bool {
        unsafe { nk_sdl_handle_event(event) }
    }

    pub(crate) fn begin(&self, title: CString, bounds: nk_rect, flags: nk_flags) -> nk_bool {
        unsafe { nk_begin(self.nk_ctx, title.as_ptr(), bounds, flags) }
    }

    pub(crate) fn end(&self) {
        unsafe {
            nk_end(self.nk_ctx);
        }
    }

    pub(crate) fn render(&self) {
        const MAX_VERTEX_MEMORY: i32 = 512 * 1024;
        const MAX_ELEMENT_MEMORY: i32 = 128 * 1024;
        unsafe {
            nk_sdl_render(
                nk_anti_aliasing_NK_ANTI_ALIASING_ON,
                MAX_VERTEX_MEMORY,
                MAX_ELEMENT_MEMORY,
            )
        }
    }

    pub(crate) fn _is_key_released(&self, key: u32) -> bool {
        unsafe { nk_input_is_key_released(&(*self.nk_ctx).input, key) != 0 }
    }

    pub(crate) fn combo_begin_label(&self, label: CString) -> bool {
        unsafe {
            nk_combo_begin_label(
                self.nk_ctx,
                label.as_ptr(),
                nk_vec2(nk_widget_width(self.nk_ctx), 300.0),
            ) != 0
        }
    }

    pub(crate) fn combo_end(&self) {
        unsafe {
            nk_combo_end(self.nk_ctx);
        }
    }

    pub(crate) fn window_set_bounds(&self, title: CString, bounds: nk_rect) {
        unsafe {
            nk_window_set_bounds(self.nk_ctx, title.as_ptr(), bounds);
        }
    }

    pub(crate) fn set_font(&self, font: Font) {
        let idx = Font::ALL.iter().position(|f| *f == font).unwrap_or(0);
        unsafe {
            nk_style_set_font(self.nk_ctx, &(*self.fonts[idx]).handle);
        }
    }

    pub(crate) fn push_heading_font(&self) {
        unsafe {
            nk_style_push_font(self.nk_ctx, &(*self.heading_font).handle);
        }
    }

    pub(crate) fn push_font(&self, font: Font) {
        let idx = Font::ALL.iter().position(|f| *f == font).unwrap_or(0);
        unsafe {
            nk_style_push_font(self.nk_ctx, &(*self.fonts[idx]).handle);
        }
    }

    pub(crate) fn pop_font(&self) {
        unsafe {
            nk_style_pop_font(self.nk_ctx);
        }
    }

    pub(crate) fn tree_push(&self, title: CString) -> bool {
        unsafe {
            let ptr = title.as_ptr();
            let len = title.as_bytes().len() as i32;
            nk_tree_push_hashed(
                self.nk_ctx,
                nk_tree_type_NK_TREE_NODE,
                ptr,
                nk_collapse_states_NK_MINIMIZED,
                ptr,
                len,
                0,
            ) != 0
        }
    }

    pub(crate) fn tree_pop(&self) {
        unsafe {
            nk_tree_pop(self.nk_ctx);
        }
    }

    pub(crate) fn color_picker(&self, r: f32, g: f32, b: f32, a: f32) -> (f32, f32, f32, f32) {
        unsafe {
            let color = nk_colorf { r, g, b, a };
            let result = nk_color_picker(self.nk_ctx, color, nk_color_format_NK_RGBA);
            (result.r, result.g, result.b, result.a)
        }
    }

    pub(crate) fn apply_palette(&self, palette: &Palette) {
        palette.apply(self.nk_ctx);
    }

    /// Leave `cols` slots empty. Not nk_spacing, which allocates the next row as soon as the
    /// skipped slots finish the current one, adding a blank row.
    pub(crate) fn spacing(&self, cols: i32) {
        for _ in 0..cols {
            unsafe {
                nk_spacer(self.nk_ctx);
            }
        }
    }

    /// Sidebar style button: left aligned, flat, and when `active` tinted with an accent bar on
    /// its left edge
    pub(crate) fn nav_button(&self, label: CString, active: bool, p: &Palette) -> bool {
        unsafe {
            let ctx = self.nk_ctx;
            let saved = (*ctx).style.button;
            let b = &mut (*ctx).style.button;
            let (bg, fg) = if active {
                (p.widget_active, p.accent)
            } else {
                (p.panel, p.text_dim)
            };
            b.normal = nk_style_item_color(bg.into());
            b.hover = nk_style_item_color(if active { bg } else { p.widget }.into());
            b.active = nk_style_item_color(p.widget_active.into());
            b.border = 0.0;
            b.text_normal = fg.into();
            b.text_hover = if active { p.accent } else { p.text }.into();
            b.text_active = p.accent.into();
            b.text_alignment = nk_text_alignment_NK_TEXT_LEFT;
            b.padding.x = 14.0;

            let bounds = nk_widget_bounds(ctx);
            let clicked = nk_button_label(ctx, label.as_ptr()) != 0;
            if active {
                let bar = nk_rect(bounds.x, bounds.y, 3.0, bounds.h);
                nk_fill_rect(nk_window_get_canvas(ctx), bar, 0.0, p.accent.into());
            }

            (*ctx).style.button = saved;
            clicked
        }
    }

    /// `group_begin` with a background other than the window's
    pub(crate) fn group_begin_bg(&self, title: CString, flags: nk_flags, bg: Color) -> bool {
        unsafe {
            let w = &mut (*self.nk_ctx).style.window;
            let saved = w.fixed_background;
            w.fixed_background = nk_style_item_color(bg.into());
            let open = nk_group_begin(self.nk_ctx, title.as_ptr(), flags) != 0;
            (*self.nk_ctx).style.window.fixed_background = saved;
            open
        }
    }

    /// Fill the bounds of the next widget slot with `color` and consume the slot
    pub(crate) fn fill_slot(&self, color: Color) {
        unsafe {
            let bounds = nk_widget_bounds(self.nk_ctx);
            nk_fill_rect(nk_window_get_canvas(self.nk_ctx), bounds, 0.0, color.into());
            nk_spacer(self.nk_ctx);
        }
    }

    pub(crate) fn push_group_padding(&self, x: f32, y: f32) {
        unsafe {
            let ctx = self.nk_ctx;
            nk_style_push_vec2(ctx, &mut (*ctx).style.window.group_padding, nk_vec2(x, y));
        }
    }

    pub(crate) fn pop_vec2(&self) {
        unsafe {
            nk_style_pop_vec2(self.nk_ctx);
        }
    }

    /// Width of `text` in pixels with the current font
    pub(crate) fn text_width(&self, text: &str) -> f32 {
        unsafe {
            let font = (*self.nk_ctx).style.font;
            match (*font).width {
                Some(width) => width(
                    (*font).userdata,
                    (*font).height,
                    text.as_ptr() as *const _,
                    text.len() as i32,
                ),
                None => 0.0,
            }
        }
    }

    /// Width available to the label of a combo placed in the next widget slot, mirroring the
    /// layout in nk_combo_begin_text
    pub(crate) fn combo_label_width(&self) -> f32 {
        unsafe {
            let header = nk_widget_bounds(self.nk_ctx);
            let c = &(*self.nk_ctx).style.combo;
            let button_x = header.x + header.w - header.h - c.button_padding.x;
            let label_x = header.x + c.content_padding.x;
            button_x - (c.content_padding.x + c.spacing.x) - label_x
        }
    }
}
