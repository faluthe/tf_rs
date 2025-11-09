use std::ffi::{CString, c_void};

use nuklear_sys::{SDL_Event, SDL_GL_MakeCurrent, SDL_Window};

use crate::{
    Key, Rect,
    context::Context,
    flags::{PanelFlags, TextAlignment},
};

static mut DO_DRAW: bool = false;

pub struct Nuklear {
    window: *mut SDL_Window,
    context: &'static Context,
}

impl Nuklear {
    pub fn get_or_init(window: *mut c_void) -> Self {
        let window = window as *mut SDL_Window;
        let context = Context::get_or_init(window);

        unsafe {
            SDL_GL_MakeCurrent(window, context.new_ctx);
        }

        Nuklear { window, context }
    }

    pub fn should_draw() -> bool {
        unsafe { DO_DRAW }
    }

    pub fn begin<T: Into<Vec<u8>>>(&self, title: T, flags: PanelFlags, size: Rect) -> bool {
        self.context
            .begin(CString::new(title).unwrap(), size.into(), flags.bits())
            != 0
    }

    pub fn row_dynamic(&self, height: f32, cols: i32) -> &Self {
        self.context.row_dynamic(height, cols);
        self
    }

    pub fn label<T: Into<Vec<u8>>>(&self, text: T, alignment: TextAlignment) -> &Self {
        self.context
            .label(CString::new(text).unwrap(), alignment as u32);
        self
    }

    pub fn checkbox<T: Into<Vec<u8>>>(&self, text: T, active: *mut i32) -> &Self {
        self.context
            .checkbox_label(CString::new(text).unwrap(), active);
        self
    }

    pub fn end(&self) {
        self.context.end();
    }

    pub fn render(&self) {
        self.context.render();
        unsafe { SDL_GL_MakeCurrent(self.window, self.context.og_ctx) };
    }

    pub fn input_begin(&self) {
        self.context.input_begin();
    }

    pub fn input_end(&self) {
        self.context.input_end();
    }

    pub fn handle_event(event: *mut c_void) -> bool {
        Context::handle_event(event as _) != 0
    }

    pub fn capture_input(event: *mut c_void) {
        let event = event as *mut SDL_Event;
        unsafe { (*event).type_ = 0 };
    }

    pub fn is_draw_key_released(&self, key: Key) -> bool {
        if self.context.is_key_released(key as u32) {
            unsafe {
                DO_DRAW = !DO_DRAW;
            }
            return true;
        }
        false
    }
}
