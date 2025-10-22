use std::{ffi::c_void, ptr};

use log::info;

use crate::{hooks::Hooks, nuklear::*};

static mut NK_CTX: *mut nk_context = ptr::null_mut();
static mut OG_CTX: SDL_GLContext = ptr::null_mut();
static mut NEW_CTX: SDL_GLContext = ptr::null_mut();

pub extern "C" fn hk_swap_window(window: *mut c_void) -> i32 {
    info!("hk_swap_window called with window: {:?}", window);

    let window = window as *mut SDL_Window;

    unsafe {
        if NK_CTX.is_null() {
            OG_CTX = SDL_GL_GetCurrentContext();
            NEW_CTX = SDL_GL_CreateContext(window);

            let err = glewInit();
            if err != GLEW_OK {
                panic!("Failed to initialize GLEW");
            }

            NK_CTX = nk_sdl_init(window);

            // set_style(NK_CTX);

            let mut atlas: *mut nk_font_atlas = ptr::null_mut();
            let p_atlas: *mut *mut nk_font_atlas = &mut atlas;
            nk_sdl_font_stash_begin(p_atlas);
            nk_sdl_font_stash_end();
        }

        SDL_GL_MakeCurrent(window, NEW_CTX);

        {
            if nk_begin(
                NK_CTX,
                "TF_RS\0".as_ptr() as *const i8,
                nk_rect(200.0, 200.0, 500.0, 600.0),
                nk_panel_flags_NK_WINDOW_BORDER
                    | nk_panel_flags_NK_WINDOW_MOVABLE
                    | nk_panel_flags_NK_WINDOW_TITLE,
            ) != 0
            {
                nk_layout_row_dynamic(NK_CTX, 40.0, 1);
                nk_label(
                    NK_CTX,
                    "Welcome to TF_RS!\0".as_ptr() as *const i8,
                    nk_text_alignment_NK_TEXT_LEFT,
                );
            }
            nk_end(NK_CTX);
        }

        const MAX_VERTEX_MEMORY: i32 = 512 * 1024;
        const MAX_ELEMENT_MEMORY: i32 = 128 * 1024;
        nk_sdl_render(
            nk_anti_aliasing_NK_ANTI_ALIASING_ON,
            MAX_VERTEX_MEMORY,
            MAX_ELEMENT_MEMORY,
        );

        SDL_GL_MakeCurrent(window, OG_CTX);

        nk_input_begin(NK_CTX);
        let rc = Hooks::swap_window()
            .original
            .call_swap_window(window as _)
            .expect("Invalid SwapWindow function signature");
        nk_input_end(NK_CTX);

        rc
    }
}
