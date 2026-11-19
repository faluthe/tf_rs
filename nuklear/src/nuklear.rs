use std::{
    ffi::{CString, c_void},
    mem, ptr, slice,
    sync::{
        RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use nuklear_sys::{
    SDL_BUTTON_LEFT, SDL_BUTTON_X2, SDL_Event, SDL_EventType, SDL_GL_MakeCurrent,
    SDL_GetKeyboardState, SDL_GetMouseState, SDL_Scancode, SDL_Window,
};

use crate::{
    Rect,
    context::Context,
    flags::{EditFlags, LayoutFormat, PanelFlags, TextAlignment},
    input::Input,
    icon::Icon,
    style::{Color, Font, Palette},
};

/// Scroll offset of a group, layout compatible with `nk_scroll`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Scroll {
    pub x: u32,
    pub y: u32,
}

static DO_DRAW: AtomicBool = AtomicBool::new(false);
/// Palette last applied with `set_palette`, used by the custom widgets
static PALETTE: RwLock<Option<Palette>> = RwLock::new(None);

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
        DO_DRAW.load(Ordering::Relaxed)
    }

    pub fn set_draw(val: bool) {
        DO_DRAW.store(val, Ordering::Relaxed);
    }

    pub fn is_delete_keyup(event: *mut c_void) -> bool {
        unsafe {
            let event = event as *mut SDL_Event;
            (*event).type_ == SDL_EventType::SDL_KEYUP as u32
                && (*event).key.keysym.scancode == SDL_Scancode::SDL_SCANCODE_DELETE
        }
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

    pub fn horizontal_separator(&self, thickness: f32) -> &Self {
        let c = self.palette().map_or(Color::hex(0x505050), |p| p.border);
        self.row_dynamic(thickness, 1);
        self.context.rule_horizontal(c.r, c.g, c.b, c.a, 0);
        self
    }

    /// Bounds of the current window, call between `begin` and `end`.
    pub fn window_get_bounds(&self) -> Rect {
        self.context.window_get_bounds().into()
    }

    pub fn get_content_region(&self) -> (f32, f32) {
        self.context.window_get_content_region()
    }

    pub fn group_begin<T: Into<Vec<u8>>>(&self, title: T, flags: PanelFlags) -> bool {
        self.context
            .group_begin(CString::new(title).unwrap(), flags.bits())
    }

    /// Like `group_begin`, but the scroll offset lives in `scroll` so the caller can read and
    /// set it. Nuklear keeps a pointer to `scroll` until `group_end`, so it must not move or be
    /// dropped before then.
    pub fn group_scrolled_begin<T: Into<Vec<u8>>>(
        &self,
        title: T,
        flags: PanelFlags,
        scroll: &mut Scroll,
    ) -> bool {
        self.context.group_scrolled_begin(
            CString::new(title).unwrap(),
            (scroll as *mut Scroll).cast(),
            flags.bits(),
        )
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

    pub fn single_select_combo(&self, items: &[&str], selected: &mut usize) -> &Self {
        let header = items.get(*selected).copied().unwrap_or("None");

        if self
            .context
            .combo_begin_label(CString::new(header).unwrap())
        {
            for (i, item) in items.iter().enumerate() {
                let mut sel = if *selected == i { 1i32 } else { 0 };
                self.context.row_dynamic(25.0, 1);
                self.context.selectable_label(
                    CString::new(*item).unwrap(),
                    TextAlignment::CENTER as u32,
                    &mut sel,
                );
                if sel != 0 {
                    *selected = i;
                }
            }
            self.context.combo_end();
        }

        self
    }

    pub fn multi_select_combo(&self, items: &[&str], selected: &mut [&mut bool]) -> &Self {
        assert_eq!(items.len(), selected.len());

        let names: Vec<&str> = items
            .iter()
            .zip(selected.iter())
            .filter_map(|(item, s)| (**s).then_some(*item))
            .collect();
        let header = self.multi_select_preview(&names);

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

    /// Comma separated list of `names` that fits in the next combo slot, e.g.
    /// "Boxes, Names". Trailing names that don't fit are summarized as "+N".
    fn multi_select_preview(&self, names: &[&str]) -> String {
        if names.is_empty() {
            return "None".to_string();
        }

        let avail = self.context.combo_label_width();
        let fits = |s: &str| self.context.text_width(s) <= avail;

        let all = names.join(", ");
        if fits(&all) {
            return all;
        }
        for shown in (1..names.len()).rev() {
            let text = format!("{}, +{}", names[..shown].join(", "), names.len() - shown);
            if fits(&text) {
                return text;
            }
        }
        format!("{} selected", names.len())
    }

    pub fn window_set_bounds(&self, window: &str, rect: Rect) {
        self.context
            .window_set_bounds(CString::new(window).unwrap(), rect.into());
    }

    /// Restyle every widget from `palette`. Only does work when the palette changed.
    pub fn set_palette(&self, palette: &Palette) {
        let mut cur = PALETTE.write().unwrap_or_else(|e| e.into_inner());
        if cur.as_ref() != Some(palette) {
            self.context.apply_palette(palette);
            *cur = Some(*palette);
        }
    }

    pub fn palette(&self) -> Option<Palette> {
        *PALETTE.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_font(&self, font: Font) {
        self.context.set_font(font);
    }

    /// Draw everything until `pop_font` with `font`
    pub fn push_font(&self, font: Font) {
        self.context.push_font(font);
    }

    pub fn pop_font(&self) {
        self.context.pop_font();
    }

    /// Skip `cols` slots of the current row
    pub fn spacing(&self, cols: i32) -> &Self {
        self.context.spacing(cols);
        self
    }

    /// Label in the theme's dimmed text color
    pub fn dim_label<T: Into<Vec<u8>>>(&self, text: T, alignment: TextAlignment) -> &Self {
        match self.palette() {
            Some(p) => {
                let c = p.text_dim;
                self.colored_label(text, alignment, c.r, c.g, c.b, c.a)
            }
            None => self.label(text, alignment),
        }
    }

    /// Bold uppercase heading in the accent color with a rule under it. Pass `first = false`
    /// to leave a gap above it so sections read as separate blocks.
    pub fn section(&self, title: &str, first: bool) -> &Self {
        self.section_impl(title, first, None);
        self
    }

    /// `section` with a small icon button at the right end of the heading. Returns whether the
    /// button was clicked.
    pub fn section_with_icon(&self, title: &str, first: bool, icon: Icon) -> bool {
        self.section_impl(title, first, Some(icon))
    }

    fn section_impl(&self, title: &str, first: bool, icon: Option<Icon>) -> bool {
        const HEIGHT: f32 = 16.0;

        let p = self.palette();
        if !first {
            self.row_dynamic(16.0, 1).spacing(1);
        }
        self.context.push_heading_font();
        if icon.is_some() {
            self.context.row_fill_then_static(HEIGHT, HEIGHT);
        } else {
            self.row_dynamic(HEIGHT, 1);
        }
        match p {
            Some(p) => {
                let c = p.accent;
                self.colored_label(title.to_uppercase(), TextAlignment::LEFT, c.r, c.g, c.b, c.a);
            }
            None => {
                self.label(title.to_uppercase(), TextAlignment::LEFT);
            }
        }
        self.context.pop_font();
        let clicked = match (icon, p) {
            (Some(icon), Some(p)) => self.context.icon_button(icon, &p),
            (Some(_), None) => self.button_label("R"),
            (None, _) => false,
        };
        let rule = p.map_or(Color::hex(0x505050), |p| p.border);
        self.row_dynamic(1.0, 1);
        self.context.fill_slot(rule);
        clicked
    }

    /// Flat left aligned navigation button, highlighted with an accent bar when `active`
    pub fn nav_button<T: Into<Vec<u8>>>(&self, label: T, active: bool) -> bool {
        match self.palette() {
            Some(p) => self
                .context
                .nav_button(CString::new(label).unwrap(), active, &p),
            None => self.button_label(label),
        }
    }

    /// Padding for groups begun until `pop_group_padding`. Nuklear reads it while laying out
    /// every row, so pop it only after the group's `group_end`.
    pub fn push_group_padding(&self, x: f32, y: f32) {
        self.context.push_group_padding(x, y);
    }

    pub fn pop_group_padding(&self) {
        self.context.pop_vec2();
    }

    /// `group_begin` with its own background color, e.g. for a sidebar
    pub fn group_begin_bg<T: Into<Vec<u8>>>(&self, title: T, flags: PanelFlags, bg: Color) -> bool {
        self.context
            .group_begin_bg(CString::new(title).unwrap(), flags.bits(), bg)
    }

    pub fn tree_push<T: Into<Vec<u8>>>(&self, title: T) -> bool {
        self.context.tree_push(CString::new(title).unwrap())
    }

    pub fn tree_pop(&self) {
        self.context.tree_pop();
    }

    pub fn color_picker(&self, r: &mut f32, g: &mut f32, b: &mut f32, a: &mut f32) -> &Self {
        (*r, *g, *b, *a) = self.context.color_picker(*r, *g, *b, *a);
        self
    }
}
