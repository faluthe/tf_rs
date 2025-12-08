use std::{ffi::CString, sync::OnceLock};

use nuklear_sys::{
    GLEW_OK, SDL_Event, SDL_GL_CreateContext, SDL_GL_GetCurrentContext, SDL_GLContext, SDL_Window,
    glewInit, nk_anti_aliasing_NK_ANTI_ALIASING_ON, nk_begin, nk_bool, nk_button_label,
    nk_checkbox_label, nk_color, nk_combo_begin_label, nk_combo_end, nk_context,
    nk_edit_string_zero_terminated, nk_end, nk_filter_default, nk_flags, nk_font_atlas,
    nk_group_begin, nk_group_end, nk_input_begin, nk_input_end, nk_input_is_key_released, nk_label,
    nk_layout_row_begin, nk_layout_row_dynamic, nk_layout_row_end, nk_layout_row_push, nk_rect,
    nk_rule_horizontal, nk_sdl_font_stash_begin, nk_sdl_font_stash_end, nk_sdl_handle_event,
    nk_sdl_init, nk_sdl_render, nk_selectable_label, nk_slider_int, nk_vec2, nk_widget_width,
    nk_window_get_content_region,
};

static CONTEXT: OnceLock<Context> = OnceLock::new();

#[derive(Clone, Copy)]
pub(crate) struct Context {
    pub nk_ctx: *mut nk_context,
    pub og_ctx: SDL_GLContext,
    pub new_ctx: SDL_GLContext,
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
            nk_sdl_font_stash_end();

            Context {
                nk_ctx,
                og_ctx,
                new_ctx,
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

    pub(crate) fn window_get_content_region(&self) -> (f32, f32) {
        unsafe {
            let rect = nk_window_get_content_region(self.nk_ctx);
            (rect.w, rect.h)
        }
    }

    pub(crate) fn group_begin(&self, title: CString, flags: nk_flags) -> bool {
        unsafe { nk_group_begin(self.nk_ctx, title.as_ptr(), flags) != 0 }
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

    pub(crate) fn is_key_released(&self, key: u32) -> bool {
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
}
