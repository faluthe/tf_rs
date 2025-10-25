use std::ffi::{CString, c_void};

use nuklear_sys::{SDL_Event, SDL_GL_MakeCurrent, SDL_Window};

use crate::{
    Rect,
    context::Context,
    flags::{PanelFlags, TextAlignment},
};

pub struct Nuklear {
    window: *mut SDL_Window,
    began: bool,
}

impl Nuklear {
    pub fn begin<T: Into<Vec<u8>>>(
        title: T,
        flags: PanelFlags,
        size: Rect,
        window: *mut c_void,
    ) -> Self {
        let window = window as *mut SDL_Window;
        let c = Context::get_or_init(window);

        unsafe {
            SDL_GL_MakeCurrent(window, c.new_ctx);
        }

        let began = c.begin(CString::new(title).unwrap(), size.into(), flags.bits()) != 0;

        Nuklear { window, began }
    }

    pub fn row_dynamic(&self, height: f32, cols: i32) -> &Self {
        if self.began {
            Context::row_dynamic(self.window, height, cols);
        }
        self
    }

    pub fn label(&self, text: CString, alignment: TextAlignment) -> &Self {
        if self.began {
            Context::label(self.window, text, alignment.bits())
        }
        self
    }

    pub fn render(&self) {
        let c = Context::get_or_init(self.window);
        c.end();
        c.render();
        unsafe { SDL_GL_MakeCurrent(self.window, c.og_ctx) };
    }

    pub fn input_begin(&self) {
        // Does not need began check
        Context::input_begin(self.window);
    }

    pub fn input_end(&self) {
        // Does not need began check
        Context::input_end(self.window);
    }

    pub fn handle_event(event: *mut c_void) {
        let event = event as *mut SDL_Event;
        if Context::handle_event(event) != 0 {
            unsafe { (*event).type_ = 0 };
        }
    }
}
