use std::{ffi::CString, sync::OnceLock};

use nuklear_sys::{
    GLEW_OK, SDL_Event, SDL_GL_CreateContext, SDL_GL_GetCurrentContext, SDL_GLContext, SDL_Window,
    glewInit, nk_anti_aliasing_NK_ANTI_ALIASING_ON, nk_begin, nk_bool, nk_button_label,
    nk_buttons_NK_BUTTON_LEFT, nk_checkbox_label, nk_collapse_states_NK_MINIMIZED, nk_color,
    nk_combo_begin_color, nk_combo_begin_label, nk_combo_end, nk_context, nk_draw_image,
    nk_edit_string_zero_terminated, nk_end, nk_fill_rect, nk_fill_rect_multi_color,
    nk_filter_default, nk_flags, nk_font, nk_font_atlas, nk_font_atlas_add_from_memory,
    nk_group_begin, nk_group_end, nk_group_scrolled_begin, nk_hsv_f, nk_hsva_colorfv, nk_image,
    nk_input_begin, nk_input_end, nk_input_has_mouse_click_down_in_rect, nk_input_is_key_released,
    nk_input_is_mouse_hovering_rect, nk_label, nk_label_colored, nk_layout_row_begin,
    nk_layout_row_dynamic, nk_layout_row_end, nk_layout_row_push, nk_layout_row_template_begin,
    nk_layout_row_template_end, nk_layout_row_template_push_dynamic,
    nk_layout_row_template_push_static, nk_rect, nk_rule_horizontal, nk_scroll,
    nk_sdl_font_stash_begin, nk_sdl_font_stash_end, nk_sdl_handle_event, nk_sdl_init,
    nk_sdl_render, nk_selectable_label, nk_slider_int, nk_spacer, nk_stroke_line, nk_stroke_rect,
    nk_style_item_color, nk_style_item_hide, nk_style_pop_font, nk_style_pop_vec2,
    nk_style_push_font, nk_style_push_vec2, nk_style_set_font, nk_text_alignment_NK_TEXT_CENTERED,
    nk_text_alignment_NK_TEXT_LEFT, nk_tree_pop, nk_tree_push_hashed, nk_tree_type_NK_TREE_NODE,
    nk_vec2, nk_widget, nk_widget_bounds, nk_widget_layout_states_NK_WIDGET_DISABLED,
    nk_widget_layout_states_NK_WIDGET_INVALID, nk_widget_layout_states_NK_WIDGET_ROM,
    nk_widget_width, nk_window_get_bounds, nk_window_get_canvas, nk_window_get_content_region,
    nk_window_set_bounds,
};

use crate::{
    icon::{ICON_SIZE, Icon},
    style::{Color, FONT_HEADING, Font, HEADING_SIZE, Palette},
};

static CONTEXT: OnceLock<Context> = OnceLock::new();

/// Side of a swatch in a combo header
const SWATCH: f32 = 12.0;
/// Gap before each swatch in a combo header
const SWATCH_GAP: f32 = 4.0;

#[derive(Clone, Copy)]
pub(crate) struct Context {
    pub nk_ctx: *mut nk_context,
    pub og_ctx: SDL_GLContext,
    pub new_ctx: SDL_GLContext,
    /// Indexed like `Font::ALL`
    fonts: [*mut nk_font; Font::ALL.len()],
    heading_font: *mut nk_font,
    /// Indexed like `Icon::ALL`, None if the icon failed to load
    icons: [Option<nk_image>; Icon::ALL.len()],
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

            let icons = Icon::ALL.map(Icon::load);

            // Open the first input window so poll_event calls have a valid begin/end block
            nk_input_begin(nk_ctx);

            Context {
                nk_ctx,
                og_ctx,
                new_ctx,
                fonts,
                heading_font,
                icons,
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

    /// `combo_begin_label` whose popup is at least `min_width` wide and at most `height` tall
    pub(crate) fn combo_begin_label_sized(
        &self,
        label: CString,
        min_width: f32,
        height: f32,
    ) -> bool {
        unsafe {
            let width = nk_widget_width(self.nk_ctx).max(min_width);
            nk_combo_begin_label(self.nk_ctx, label.as_ptr(), nk_vec2(width, height)) != 0
        }
    }

    /// Bounds of the next widget slot, without consuming it
    pub(crate) fn widget_bounds(&self) -> nk_rect {
        unsafe { nk_widget_bounds(self.nk_ctx) }
    }

    /// Width taken by `count` swatches drawn by `combo_swatches`, including the gap before them
    pub(crate) fn combo_swatches_width(&self, count: usize) -> f32 {
        if count == 0 {
            return 0.0;
        }
        count as f32 * (SWATCH + SWATCH_GAP)
    }

    /// Draw small swatches, right aligned against the arrow of the combo whose header was at
    /// `header`. `None` draws an empty outline for an option that is turned off.
    pub(crate) fn combo_swatches(&self, header: nk_rect, swatches: &[Option<Color>], p: &Palette) {
        unsafe {
            let ctx = self.nk_ctx;
            let c = &(*ctx).style.combo;
            let button_x = header.x + header.w - header.h - c.button_padding.x;
            let canvas = nk_window_get_canvas(ctx);
            let y = (header.y + (header.h - SWATCH) / 2.0).round();
            let mut x = (button_x - c.spacing.x - self.combo_swatches_width(swatches.len())
                + SWATCH_GAP)
                .round();
            for swatch in swatches {
                let rect = nk_rect(x, y, SWATCH, SWATCH);
                match swatch {
                    Some(color) => {
                        nk_fill_rect(canvas, rect, 0.0, color.opaque().into());
                        nk_stroke_rect(canvas, rect, 0.0, 1.0, p.border.into());
                    }
                    None => nk_stroke_rect(canvas, rect, 0.0, 1.0, p.text_dim.into()),
                }
                x += SWATCH + SWATCH_GAP;
            }
        }
    }

    /// Small chevron in the arrow slot of the combo whose header was at `header`. Palettes turn
    /// nuklear's own triangle off, so this draws the arrow instead.
    pub(crate) fn combo_chevron(&self, header: nk_rect, p: &Palette) {
        const HALF_W: f32 = 4.0;
        const HALF_H: f32 = 2.0;
        unsafe {
            let ctx = self.nk_ctx;
            let c = &(*ctx).style.combo;
            let button_w = header.h - 2.0 * c.button_padding.y;
            let button_x = header.x + header.w - header.h - c.button_padding.x;
            let cx = (button_x + button_w / 2.0).round();
            let cy = (header.y + header.h / 2.0).round();
            let hovered = nk_input_is_mouse_hovering_rect(&(*ctx).input, header) != 0;
            let color = if hovered { p.accent } else { p.text_dim };

            let canvas = nk_window_get_canvas(ctx);
            let (left, tip, right) = (
                (cx - HALF_W, cy - HALF_H),
                (cx, cy + HALF_H),
                (cx + HALF_W, cy - HALF_H),
            );
            for (a, b) in [(left, tip), (tip, right)] {
                nk_stroke_line(canvas, a.0, a.1, b.0, b.1, 1.5, color.into());
            }
        }
    }

    /// Button filled with `color`, outlined in the accent color when `active`
    pub(crate) fn swatch_button(&self, color: Color, active: bool, p: &Palette) -> bool {
        unsafe {
            let ctx = self.nk_ctx;
            let bounds = nk_widget_bounds(ctx);

            let saved = (*ctx).style.button;
            let b = &mut (*ctx).style.button;
            b.normal = nk_style_item_hide();
            b.hover = nk_style_item_hide();
            b.active = nk_style_item_hide();
            b.border = 0.0;
            let empty = CString::default();
            let clicked = nk_button_label(ctx, empty.as_ptr()) != 0;
            (*ctx).style.button = saved;

            let hovered = nk_input_is_mouse_hovering_rect(&(*ctx).input, bounds) != 0;
            let rect = nk_rect(
                bounds.x + 2.0,
                bounds.y + 3.0,
                bounds.w - 4.0,
                bounds.h - 6.0,
            );
            let canvas = nk_window_get_canvas(ctx);
            nk_fill_rect(canvas, rect, 0.0, color.opaque().into());
            let (outline, thickness) = if active {
                (p.accent, 2.0)
            } else if hovered {
                (p.text_dim, 1.0)
            } else {
                (p.border, 1.0)
            };
            nk_stroke_rect(canvas, rect, 0.0, thickness, outline.into());
            clicked
        }
    }

    /// Combo whose header is a swatch of `color`, the popup is at least `min_width` wide
    pub(crate) fn combo_begin_color(&self, color: Color, min_width: f32, height: f32) -> bool {
        unsafe {
            let width = nk_widget_width(self.nk_ctx).max(min_width);
            nk_combo_begin_color(self.nk_ctx, color.into(), nk_vec2(width, height)) != 0
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

    /// Same look and layout as `nk_color_picker`, but driven by `hsva` instead of an RGB color.
    /// Nuklear's picker re-derives HSV from RGB every frame, so hue and saturation are lost the
    /// moment the color reaches black (or grey) and the cursor jumps to the red, left edge.
    /// Returns the new color if the user changed it.
    pub(crate) fn color_picker(&self, hsva: &mut [f32; 4]) -> Option<Color> {
        unsafe {
            let ctx = self.nk_ctx;
            let mut bounds = nk_rect(0.0, 0.0, 0.0, 0.0);
            let state = nk_widget(&mut bounds, ctx);
            if state == nk_widget_layout_states_NK_WIDGET_INVALID {
                return None;
            }

            let bar_w = (*(*ctx).style.font).height;
            let matrix = nk_rect(bounds.x, bounds.y, bounds.w - 2.0 * bar_w, bounds.h);
            let hue_bar = nk_rect(matrix.x + matrix.w, bounds.y, bar_w, matrix.h);
            let alpha_bar = nk_rect(hue_bar.x + hue_bar.w, bounds.y, bar_w, matrix.h);

            let mut changed = false;
            if state != nk_widget_layout_states_NK_WIDGET_ROM
                && state != nk_widget_layout_states_NK_WIDGET_DISABLED
            {
                let input = &(*ctx).input;
                let mouse = input.mouse.pos;
                // Pressed inside the rect and still held, so dragging past the edge keeps working
                let held = |r: nk_rect| {
                    nk_input_has_mouse_click_down_in_rect(input, nk_buttons_NK_BUTTON_LEFT, r, 1)
                        != 0
                };
                let frac =
                    |v: f32, start: f32, len: f32| ((v - start) / (len - 1.0)).clamp(0.0, 1.0);

                if held(matrix) {
                    hsva[1] = frac(mouse.x, matrix.x, matrix.w);
                    hsva[2] = 1.0 - frac(mouse.y, matrix.y, matrix.h);
                    changed = true;
                }
                if held(hue_bar) {
                    hsva[0] = frac(mouse.y, hue_bar.y, hue_bar.h);
                    changed = true;
                }
                if held(alpha_bar) {
                    hsva[3] = 1.0 - frac(mouse.y, alpha_bar.y, alpha_bar.h);
                    changed = true;
                }
            }

            let out = nk_window_get_canvas(ctx);
            let white = nk_color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            };
            let black = nk_color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            };
            let clear = nk_color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            };

            // Hue bar, red at both ends
            const HUES: [u32; 7] = [
                0xff0000, 0xffff00, 0x00ff00, 0x00ffff, 0x0000ff, 0xff00ff, 0xff0000,
            ];
            let seg = hue_bar.h / 6.0;
            for i in 0..6 {
                let (a, b) = (Color::hex(HUES[i]).into(), Color::hex(HUES[i + 1]).into());
                let r = nk_rect(
                    hue_bar.x,
                    hue_bar.y + i as f32 * seg + 0.5,
                    hue_bar.w,
                    seg + 0.5,
                );
                nk_fill_rect_multi_color(out, r, a, a, b, b);
            }
            let y = (hue_bar.y + hsva[0] * matrix.h + 0.5).trunc();
            nk_stroke_line(
                out,
                hue_bar.x - 1.0,
                y,
                hue_bar.x + hue_bar.w + 2.0,
                y,
                1.0,
                white,
            );

            // Alpha bar
            nk_fill_rect_multi_color(out, alpha_bar, white, white, black, black);
            let y = (alpha_bar.y + (1.0 - hsva[3].clamp(0.0, 1.0)) * matrix.h + 0.5).trunc();
            nk_stroke_line(
                out,
                alpha_bar.x - 1.0,
                y,
                alpha_bar.x + alpha_bar.w + 2.0,
                y,
                1.0,
                white,
            );

            // Saturation/value square
            let hue = nk_hsv_f(hsva[0], 1.0, 1.0);
            nk_fill_rect_multi_color(out, matrix, white, hue, hue, white);
            nk_fill_rect_multi_color(out, matrix, clear, clear, black, black);

            // Crosshair
            const ARM: f32 = 7.0;
            let x = (matrix.x + hsva[1] * matrix.w).trunc();
            let y = (matrix.y + (1.0 - hsva[2]) * matrix.h).trunc();
            nk_stroke_line(out, x - ARM, y, x - 2.0, y, 1.0, white);
            nk_stroke_line(out, x + ARM + 1.0, y, x + 3.0, y, 1.0, white);
            nk_stroke_line(out, x, y + ARM + 1.0, x, y + 3.0, 1.0, white);
            nk_stroke_line(out, x, y - ARM, x, y - 2.0, 1.0, white);

            changed.then(|| {
                let c = nk_hsva_colorfv(hsva.as_ptr());
                Color::from_f32(c.r, c.g, c.b, c.a)
            })
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

    /// One tab of a horizontal tab bar, the active tab is underlined in the accent color
    pub(crate) fn tab_button(&self, label: CString, active: bool, p: &Palette) -> bool {
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
            b.rounding = 0.0;
            b.text_normal = fg.into();
            b.text_hover = if active { p.accent } else { p.text }.into();
            b.text_active = p.accent.into();
            b.text_alignment = nk_text_alignment_NK_TEXT_CENTERED;

            let bounds = nk_widget_bounds(ctx);
            let clicked = nk_button_label(ctx, label.as_ptr()) != 0;
            if active {
                let bar = nk_rect(bounds.x, bounds.y + bounds.h - 2.0, bounds.w, 2.0);
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

    /// `begin` with a background other than the window style's, transparent leaves a hole
    pub(crate) fn begin_bg(
        &self,
        title: CString,
        bounds: nk_rect,
        flags: nk_flags,
        bg: Color,
    ) -> bool {
        unsafe {
            let w = &mut (*self.nk_ctx).style.window;
            let saved = w.fixed_background;
            w.fixed_background = nk_style_item_color(bg.into());
            let open = nk_begin(self.nk_ctx, title.as_ptr(), bounds, flags) != 0;
            (*self.nk_ctx).style.window.fixed_background = saved;
            open
        }
    }

    /// Bounds of the next widget slot, consuming it
    pub(crate) fn reserve_slot(&self) -> nk_rect {
        unsafe {
            let bounds = nk_widget_bounds(self.nk_ctx);
            nk_spacer(self.nk_ctx);
            bounds
        }
    }

    /// How far the mouse moved this frame while dragging with the left button pressed in `rect`
    pub(crate) fn drag_x(&self, rect: nk_rect) -> Option<f32> {
        unsafe {
            let input = &(*self.nk_ctx).input;
            let dragging =
                nk_input_has_mouse_click_down_in_rect(input, nk_buttons_NK_BUTTON_LEFT, rect, 1)
                    != 0;
            dragging.then_some(input.mouse.delta.x)
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

    /// Row where the first slot takes all the space left over by a trailing `width` slot
    pub(crate) fn row_fill_then_static(&self, height: f32, width: f32) {
        unsafe {
            nk_layout_row_template_begin(self.nk_ctx, height);
            nk_layout_row_template_push_dynamic(self.nk_ctx);
            nk_layout_row_template_push_static(self.nk_ctx, width);
            nk_layout_row_template_end(self.nk_ctx);
        }
    }

    /// Borderless button showing only `icon`, dim when idle and accent colored on hover
    pub(crate) fn icon_button(&self, icon: Icon, p: &Palette) -> bool {
        unsafe {
            let ctx = self.nk_ctx;
            let bounds = nk_widget_bounds(ctx);
            let hovered = nk_input_is_mouse_hovering_rect(&(*ctx).input, bounds) != 0;

            let saved = (*ctx).style.button;
            let b = &mut (*ctx).style.button;
            b.normal = nk_style_item_hide();
            b.hover = nk_style_item_hide();
            b.active = nk_style_item_hide();
            b.border = 0.0;
            let empty = CString::default();
            let clicked = nk_button_label(ctx, empty.as_ptr()) != 0;
            (*ctx).style.button = saved;

            let color = if hovered { p.accent } else { p.text_dim };
            if let Some(img) = &self.icons[icon.index()] {
                // Whole pixels at the icon's native size so it isn't filtered blurry
                let size = ICON_SIZE as f32;
                let x = (bounds.x + (bounds.w - size) / 2.0).round();
                let y = (bounds.y + (bounds.h - size) / 2.0).round();
                let rect = nk_rect(x, y, size, size);
                nk_draw_image(nk_window_get_canvas(ctx), rect, img, color.into());
            }
            clicked
        }
    }
}
