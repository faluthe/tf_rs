use std::ffi::{CString, c_void};

use nuklear_sys::{
    GLEW_OK, SDL_GL_CreateContext, SDL_GL_GetCurrentContext, SDL_GL_MakeCurrent, SDL_GLContext,
    SDL_Window, glewInit, nk_anti_aliasing_NK_ANTI_ALIASING_ON, nk_begin, nk_context, nk_end,
    nk_font_atlas, nk_input_begin, nk_input_end, nk_label, nk_layout_row_dynamic, nk_rect,
    nk_sdl_font_stash_begin, nk_sdl_font_stash_end, nk_sdl_init, nk_sdl_render,
};

// TODO: Is this thread safe?
static mut CONTEXT: Option<Context> = None;

pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy)]
struct Context {
    nk_ctx: *mut nk_context,
    og_ctx: SDL_GLContext,
    new_ctx: SDL_GLContext,
}

pub struct Nuklear {
    window: *mut SDL_Window,
    begin: bool,
}

impl Nuklear {
    pub fn begin<T: Into<Vec<u8>>>(window: *mut c_void, size: Rect, flags: u32, title: T) -> Self {
        let mut n = Nuklear {
            window: window as *mut SDL_Window,
            begin: false,
        };

        let c = n.ctx();
        unsafe {
            SDL_GL_MakeCurrent(n.window, c.new_ctx);

            n.begin = nk_begin(
                c.nk_ctx,
                CString::new(title).unwrap().as_ptr(),
                nk_rect(size.x, size.y, size.w, size.h),
                flags,
            ) != 0;
        }

        n
    }

    pub fn row_dynamic(&self, height: f32, cols: i32) -> &Self {
        if !self.begin {
            return self;
        }

        let c = self.ctx();
        unsafe {
            nk_layout_row_dynamic(c.nk_ctx, height, cols);
        }

        self
    }

    pub fn label(&self, text: CString, alignment: u32) -> &Self {
        if !self.begin {
            return self;
        }

        let c = self.ctx();
        unsafe {
            nk_label(c.nk_ctx, text.as_ptr(), alignment);
        }

        self
    }

    pub fn render(&self) {
        const MAX_VERTEX_MEMORY: i32 = 512 * 1024;
        const MAX_ELEMENT_MEMORY: i32 = 128 * 1024;
        let c = self.ctx();

        unsafe {
            // Called no matter what nk_begin returns
            nk_end(c.nk_ctx);

            nk_sdl_render(
                nk_anti_aliasing_NK_ANTI_ALIASING_ON,
                MAX_VERTEX_MEMORY,
                MAX_ELEMENT_MEMORY,
            );

            SDL_GL_MakeCurrent(self.window, c.og_ctx);
        }
    }

    pub fn input_begin(&self) {
        let c = self.ctx();
        unsafe {
            nk_input_begin(c.nk_ctx);
        }
    }

    pub fn input_end(&self) {
        let c = self.ctx();
        unsafe {
            nk_input_end(c.nk_ctx);
        }
    }

    /// CAUTION: This function panics!
    fn ctx(&self) -> Context {
        unsafe {
            match CONTEXT {
                None => {
                    let og_ctx = SDL_GL_GetCurrentContext();
                    let new_ctx = SDL_GL_CreateContext(self.window);

                    let err = glewInit();
                    if err != GLEW_OK {
                        panic!("Failed to initialize GLEW");
                    }

                    let nk_ctx = nk_sdl_init(self.window);

                    let mut atlas: *mut nk_font_atlas = std::ptr::null_mut();
                    let p_atlas: *mut *mut nk_font_atlas = &mut atlas;
                    nk_sdl_font_stash_begin(p_atlas);
                    nk_sdl_font_stash_end();

                    CONTEXT = Some(Context {
                        nk_ctx,
                        og_ctx,
                        new_ctx,
                    });
                    CONTEXT.unwrap()
                }
                Some(ctx) => return ctx,
            }
        }
    }
}
