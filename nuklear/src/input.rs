use nuklear_sys::{SDL_Scancode, nk_keys_NK_KEY_DEL};

#[repr(u32)]
pub enum NkKey {
    Delete = nk_keys_NK_KEY_DEL,
}

#[derive(Default)]
pub enum Input {
    Key(SDL_Scancode),
    MouseButton(u32),
    #[default]
    None,
}
