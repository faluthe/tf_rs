use std::{
    ffi::{CString, c_void},
    mem, ptr, slice,
};

use nuklear_sys::{
    SDL_BUTTON_LEFT, SDL_BUTTON_X2, SDL_Event, SDL_GL_MakeCurrent, SDL_GetKeyboardState,
    SDL_GetMouseState, SDL_Scancode, SDL_Window,
};

use crate::{
    NkKey, Rect,
    context::Context,
    flags::{EditFlags, LayoutFormat, PanelFlags, TextAlignment},
    input::Input,
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

    pub fn colored_label<T: Into<Vec<u8>>>(
        &self,
        text: T,
        alignment: TextAlignment,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) -> &Self {
        self.context
            .colored_label(CString::new(text).unwrap(), alignment as u32, r, g, b, a);
        self
    }

    pub fn checkbox<T: Into<Vec<u8>>>(&self, text: T, active: &mut bool) -> &Self {
        let mut active_i32 = if *active { 1 } else { 0 };
        self.context
            .checkbox_label(CString::new(text).unwrap(), &mut active_i32);
        *active = active_i32 != 0;
        self
    }

    pub fn button_label<T: Into<Vec<u8>>>(&self, label: T) -> bool {
        self.context.button_label(CString::new(label).unwrap())
    }

    pub fn slider_int(&self, min: i32, val: *mut i32, max: i32, step: i32) -> &Self {
        self.context.slider_int(min, val, max, step);
        self
    }

    pub fn slider_float(&self, min: f32, val: *mut f32, max: f32, step: f32) -> &Self {
        self.context.slider_float(min, val, max, step);
        self
    }

    pub fn horizontal_separator(&self, thickness: f32) -> &Self {
        self.row_dynamic(thickness, 1);
        self.context.rule_horizontal(80, 80, 80, 255, 0);
        self
    }

    pub fn get_content_region(&self) -> (f32, f32) {
        self.context.window_get_content_region()
    }

    pub fn group_begin<T: Into<Vec<u8>>>(&self, title: T, flags: PanelFlags) -> bool {
        self.context
            .group_begin(CString::new(title).unwrap(), flags.bits())
    }

    pub fn layout_row_begin(&self, fmt: LayoutFormat, row_height: f32, cols: i32) -> &Self {
        self.context.layout_row_begin(fmt as u32, row_height, cols);
        self
    }

    pub fn layout_row_push(&self, width: f32) -> &Self {
        self.context.layout_row_push(width);
        self
    }

    pub fn layout_row_end(&self) -> &Self {
        self.context.layout_row_end();
        self
    }

    pub fn group_end(&self) {
        self.context.group_end();
    }

    pub fn selectable_label<T: Into<Vec<u8>>>(
        &self,
        label: T,
        align: TextAlignment,
        selected: *mut i32,
    ) -> bool {
        self.context
            .selectable_label(CString::new(label).unwrap(), align as u32, selected)
    }

    pub fn edit_string(&self, flags: EditFlags, buffer: *mut i8, max: i32) -> &Self {
        self.context
            .edit_string_zero_terminated(flags.bits(), buffer, max);
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

    pub fn is_draw_key_released(&self, key: NkKey) -> bool {
        if self.context.is_key_released(key as u32) {
            unsafe {
                DO_DRAW = !DO_DRAW;
            }
            return true;
        }
        false
    }

    pub fn is_input_pressed(code: u32, is_mouse_button: bool) -> bool {
        if is_mouse_button {
            let state = unsafe { SDL_GetMouseState(ptr::null_mut(), ptr::null_mut()) };
            // SDL_BUTTON macro
            state & (1 << (code - 1)) != 0
        } else {
            let state = unsafe { SDL_GetKeyboardState(ptr::null_mut()) };
            unsafe { *state.add(code as usize) != 0 }
        }
    }

    pub fn get_input_pressed() -> Input {
        let state = unsafe { SDL_GetKeyboardState(ptr::null_mut()) };
        let state =
            unsafe { slice::from_raw_parts(state, SDL_Scancode::SDL_NUM_SCANCODES as usize) };

        for (i, &pressed) in state.iter().enumerate() {
            if pressed != 0 {
                return Input::Key(unsafe { mem::transmute(i as u32) });
            }
        }

        let state = unsafe { SDL_GetMouseState(ptr::null_mut(), ptr::null_mut()) };
        for i in SDL_BUTTON_LEFT..=SDL_BUTTON_X2 {
            // SDL_BUTTON macro
            if state & (1 << (i - 1)) != 0 {
                return Input::MouseButton(i);
            }
        }

        Input::None
    }

    pub fn multi_select_combo(&self, items: &[&str], selected: &mut [&mut bool]) -> &Self {
        assert_eq!(items.len(), selected.len());

        let selected_count = selected.iter().filter(|&s| **s).count();
        let header = if selected_count == 0 {
            "None".to_string()
        } else {
            format!("{} selected", selected_count)
        };

        if self
            .context
            .combo_begin_label(CString::new(header).unwrap())
        {
            for (i, item) in items.iter().enumerate() {
                let mut selected_i32 = if *selected[i] { 1 } else { 0 };
                self.context.row_dynamic(25.0, 1);
                self.context.selectable_label(
                    CString::new(*item).unwrap(),
                    TextAlignment::CENTER as u32,
                    &mut selected_i32,
                );
                *selected[i] = selected_i32 != 0;
            }
            self.context.combo_end();
        }

        self
    }

    pub fn window_set_bounds(&self, window: &str, rect: Rect) {
        self.context
            .window_set_bounds(CString::new(window).unwrap(), rect.into());
    }

    pub fn set_button_normal_color(&self, r: u8, g: u8, b: u8, a: u8) {
        self.context.set_button_normal_color(r, g, b, a);
    }

    pub fn set_button_rounding(&self, rounding: f32) {
        self.context.set_button_rounding(rounding);
    }
}
