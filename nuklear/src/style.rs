use std::{fmt, str::FromStr};

use nuklear_sys::{
    nk_color, nk_colorf, nk_colorf_hsva_fv, nk_context, nk_style_colors_NK_COLOR_BORDER,
    nk_style_colors_NK_COLOR_BUTTON, nk_style_colors_NK_COLOR_BUTTON_ACTIVE,
    nk_style_colors_NK_COLOR_BUTTON_HOVER, nk_style_colors_NK_COLOR_CHART,
    nk_style_colors_NK_COLOR_CHART_COLOR, nk_style_colors_NK_COLOR_CHART_COLOR_HIGHLIGHT,
    nk_style_colors_NK_COLOR_COMBO, nk_style_colors_NK_COLOR_COUNT, nk_style_colors_NK_COLOR_EDIT,
    nk_style_colors_NK_COLOR_EDIT_CURSOR, nk_style_colors_NK_COLOR_HEADER,
    nk_style_colors_NK_COLOR_KNOB, nk_style_colors_NK_COLOR_KNOB_CURSOR,
    nk_style_colors_NK_COLOR_KNOB_CURSOR_ACTIVE, nk_style_colors_NK_COLOR_KNOB_CURSOR_HOVER,
    nk_style_colors_NK_COLOR_PROPERTY, nk_style_colors_NK_COLOR_SCROLLBAR,
    nk_style_colors_NK_COLOR_SCROLLBAR_CURSOR, nk_style_colors_NK_COLOR_SCROLLBAR_CURSOR_ACTIVE,
    nk_style_colors_NK_COLOR_SCROLLBAR_CURSOR_HOVER, nk_style_colors_NK_COLOR_SELECT,
    nk_style_colors_NK_COLOR_SELECT_ACTIVE, nk_style_colors_NK_COLOR_SLIDER,
    nk_style_colors_NK_COLOR_SLIDER_CURSOR, nk_style_colors_NK_COLOR_SLIDER_CURSOR_ACTIVE,
    nk_style_colors_NK_COLOR_SLIDER_CURSOR_HOVER, nk_style_colors_NK_COLOR_TAB_HEADER,
    nk_style_colors_NK_COLOR_TEXT, nk_style_colors_NK_COLOR_TOGGLE,
    nk_style_colors_NK_COLOR_TOGGLE_CURSOR, nk_style_colors_NK_COLOR_TOGGLE_HOVER,
    nk_style_colors_NK_COLOR_WINDOW, nk_style_from_table, nk_style_item_color,
    nk_symbol_type_NK_SYMBOL_NONE, nk_vec2,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Same color with full alpha, for previews
    pub const fn opaque(self) -> Color {
        Color { a: 255, ..self }
    }

    pub const WHITE: Color = Color::hex(0xffffff);

    pub const fn hex(rgb: u32) -> Self {
        Color {
            r: (rgb >> 16) as u8,
            g: (rgb >> 8) as u8,
            b: rgb as u8,
            a: 255,
        }
    }

    pub(crate) fn from_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color {
            r: q(r),
            g: q(g),
            b: q(b),
            a: q(a),
        }
    }

    pub(crate) fn to_hsva(self) -> [f32; 4] {
        let f = |v: u8| v as f32 / 255.0;
        let rgba = nk_colorf {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
            a: f(self.a),
        };
        let mut hsva = [0.0; 4];
        unsafe { nk_colorf_hsva_fv(hsva.as_mut_ptr(), rgba) };
        hsva
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

/// Written as `#rrggbbaa`
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "#{:02x}{:02x}{:02x}{:02x}",
            self.r, self.g, self.b, self.a
        )
    }
}

/// Reads `#rrggbbaa` or `#rrggbb`, and also the older layout of one `r: 0.0..1.0` line per
/// channel so saved configurations from before the switch to 8 bit colors still load
impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if let Some(hex) = s.strip_prefix('#') {
            let v = u32::from_str_radix(hex, 16).map_err(|e| format!("bad color `{s}`: {e}"))?;
            return match hex.len() {
                6 => Ok(Color::hex(v)),
                8 => Ok(Color {
                    r: (v >> 24) as u8,
                    g: (v >> 16) as u8,
                    b: (v >> 8) as u8,
                    a: v as u8,
                }),
                _ => Err(format!("bad color `{s}`: expected #rrggbb or #rrggbbaa")),
            };
        }

        let (mut r, mut g, mut b, mut a) = (1.0, 1.0, 1.0, 1.0);
        for line in s.lines() {
            let Some((key, val)) = line.split_once(':') else {
                continue;
            };
            let val: f32 = val
                .trim()
                .parse()
                .map_err(|e| format!("bad color channel `{}`: {e}", line.trim()))?;
            match key.trim() {
                "r" => r = val,
                "g" => g = val,
                "b" => b = val,
                "a" => a = val,
                _ => {}
            }
        }
        Ok(Color::from_f32(r, g, b, a))
    }
}

impl From<Color> for nk_color {
    fn from(c: Color) -> Self {
        nk_color {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }
}

/// The handful of colors a theme is built from. Everything else is derived in `apply`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Main content background
    pub bg: Color,
    /// Title bar, sidebar and text fields, a shade darker than `bg`
    pub panel: Color,
    /// Idle buttons, combos, slider track, checkbox box
    pub widget: Color,
    pub widget_hover: Color,
    pub widget_active: Color,
    pub border: Color,
    pub text: Color,
    pub text_dim: Color,
    pub accent: Color,
    pub accent_hover: Color,
}

impl Palette {
    pub(crate) fn apply(&self, ctx: *mut nk_context) {
        let mut table = [nk_color::from(self.text); nk_style_colors_NK_COLOR_COUNT as usize];
        let mut set = |idx: u32, c: Color| table[idx as usize] = c.into();

        set(nk_style_colors_NK_COLOR_TEXT, self.text);
        set(nk_style_colors_NK_COLOR_WINDOW, self.bg);
        set(nk_style_colors_NK_COLOR_HEADER, self.panel);
        set(nk_style_colors_NK_COLOR_BORDER, self.border);
        set(nk_style_colors_NK_COLOR_BUTTON, self.widget);
        set(nk_style_colors_NK_COLOR_BUTTON_HOVER, self.widget_hover);
        set(nk_style_colors_NK_COLOR_BUTTON_ACTIVE, self.widget_active);
        set(nk_style_colors_NK_COLOR_TOGGLE, self.panel);
        set(nk_style_colors_NK_COLOR_TOGGLE_HOVER, self.widget);
        set(nk_style_colors_NK_COLOR_TOGGLE_CURSOR, self.accent);
        set(nk_style_colors_NK_COLOR_SELECT, self.bg);
        set(nk_style_colors_NK_COLOR_SELECT_ACTIVE, self.widget_active);
        set(nk_style_colors_NK_COLOR_SLIDER, self.widget);
        set(nk_style_colors_NK_COLOR_SLIDER_CURSOR, self.accent);
        set(
            nk_style_colors_NK_COLOR_SLIDER_CURSOR_HOVER,
            self.accent_hover,
        );
        set(
            nk_style_colors_NK_COLOR_SLIDER_CURSOR_ACTIVE,
            self.accent_hover,
        );
        set(nk_style_colors_NK_COLOR_PROPERTY, self.widget);
        set(nk_style_colors_NK_COLOR_EDIT, self.panel);
        set(nk_style_colors_NK_COLOR_EDIT_CURSOR, self.accent);
        set(nk_style_colors_NK_COLOR_COMBO, self.widget);
        set(nk_style_colors_NK_COLOR_CHART, self.widget);
        set(nk_style_colors_NK_COLOR_CHART_COLOR, self.accent);
        set(
            nk_style_colors_NK_COLOR_CHART_COLOR_HIGHLIGHT,
            self.accent_hover,
        );
        set(nk_style_colors_NK_COLOR_SCROLLBAR, self.panel);
        set(
            nk_style_colors_NK_COLOR_SCROLLBAR_CURSOR,
            self.widget_active,
        );
        set(
            nk_style_colors_NK_COLOR_SCROLLBAR_CURSOR_HOVER,
            self.text_dim,
        );
        set(
            nk_style_colors_NK_COLOR_SCROLLBAR_CURSOR_ACTIVE,
            self.accent,
        );
        set(nk_style_colors_NK_COLOR_TAB_HEADER, self.panel);
        set(nk_style_colors_NK_COLOR_KNOB, self.widget);
        set(nk_style_colors_NK_COLOR_KNOB_CURSOR, self.accent);
        set(
            nk_style_colors_NK_COLOR_KNOB_CURSOR_HOVER,
            self.accent_hover,
        );
        set(
            nk_style_colors_NK_COLOR_KNOB_CURSOR_ACTIVE,
            self.accent_hover,
        );

        let border: nk_color = self.border.into();
        let accent: nk_color = self.accent.into();
        let text: nk_color = self.text.into();

        unsafe {
            nk_style_from_table(ctx, table.as_ptr());
            let s = &mut (*ctx).style;

            // Flat, square and tight: no rounding anywhere, thin borders
            let w = &mut s.window;
            w.rounding = 0.0;
            w.border = 1.0;
            w.group_border = 1.0;
            w.combo_border = 1.0;
            w.popup_border = 1.0;
            w.border_color = border;
            w.group_border_color = border;
            w.combo_border_color = border;
            w.popup_border_color = border;
            w.padding = nk_vec2(8.0, 8.0);
            w.group_padding = nk_vec2(8.0, 6.0);
            w.combo_padding = nk_vec2(4.0, 4.0);
            w.spacing = nk_vec2(6.0, 5.0);
            w.scrollbar_size = nk_vec2(6.0, 6.0);
            w.min_row_height_padding = 4.0;
            // Nuklear places the title using only label_padding.y but sizes the header with
            // both, so the two y paddings must match for the title to sit centered
            w.header.padding = nk_vec2(8.0, 4.0);
            w.header.label_padding = nk_vec2(4.0, 4.0);
            w.header.label_normal = accent;
            w.header.label_hover = accent;
            w.header.label_active = accent;

            let b = &mut s.button;
            b.rounding = 0.0;
            b.border = 1.0;
            b.border_color = border;
            b.text_active = accent;
            b.padding = nk_vec2(6.0, 2.0);

            for t in [&mut s.checkbox, &mut s.option] {
                t.border = 1.0;
                t.border_color = border;
                t.text_hover = text;
                t.text_active = text;
                t.padding = nk_vec2(3.0, 3.0);
                t.spacing = 8.0;
            }

            s.selectable.rounding = 0.0;
            s.selectable.text_normal_active = accent;
            s.selectable.text_hover_active = accent;
            s.selectable.text_pressed_active = accent;

            let sl = &mut s.slider;
            sl.rounding = 0.0;
            sl.bar_height = 3.0;
            sl.cursor_size = nk_vec2(12.0, 12.0);
            sl.bar_filled = accent;

            for sc in [&mut s.scrollv, &mut s.scrollh] {
                sc.rounding = 0.0;
                sc.rounding_cursor = 0.0;
                sc.border = 0.0;
                sc.border_cursor = 0.0;
            }

            let c = &mut s.combo;
            c.rounding = 0.0;
            c.border = 1.0;
            c.border_color = border;
            c.button.rounding = 0.0;
            c.button.normal = nk_style_item_color(self.widget.into());
            c.button.hover = nk_style_item_color(self.widget.into());
            c.button.active = nk_style_item_color(self.widget.into());
            c.button.text_normal = self.text_dim.into();
            c.button.text_hover = accent;
            c.button.text_active = accent;
            // Drawn as a thin chevron by Context::combo_chevron instead
            c.sym_normal = nk_symbol_type_NK_SYMBOL_NONE;
            c.sym_hover = nk_symbol_type_NK_SYMBOL_NONE;
            c.sym_active = nk_symbol_type_NK_SYMBOL_NONE;

            let e = &mut s.edit;
            e.rounding = 0.0;
            e.border = 1.0;
            e.border_color = border;
            e.cursor_size = 1.0;

            let t = &mut s.tab;
            t.rounding = 0.0;
            t.border = 0.0;
            t.text = accent;
            t.node_minimize_button.rounding = 0.0;
            t.node_maximize_button.rounding = 0.0;

            s.property.rounding = 0.0;
            s.property.border_color = border;
        }
    }
}

/// Body fonts that can be switched at runtime. All are baked into the atlas at init.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Font {
    /// Clean sans serif
    Roboto,
    /// Monospace, terminal feel
    Cousine,
    /// Compact grotesque
    Karla,
    /// Nuklear's original bitmap-style font
    Proggy,
}

impl Font {
    pub const ALL: [Font; 4] = [Font::Roboto, Font::Cousine, Font::Karla, Font::Proggy];

    pub fn name(self) -> &'static str {
        match self {
            Font::Roboto => "Roboto",
            Font::Cousine => "Cousine (mono)",
            Font::Karla => "Karla",
            Font::Proggy => "Proggy (classic)",
        }
    }

    pub(crate) fn data(self) -> (&'static [u8], f32) {
        match self {
            Font::Roboto => (FONT_ROBOTO, 15.0),
            Font::Cousine => (FONT_COUSINE, 14.0),
            Font::Karla => (FONT_KARLA, 15.0),
            Font::Proggy => (FONT_PROGGY, 13.0),
        }
    }
}

// Fonts ship with the vendored nuklear sources
static FONT_ROBOTO: &[u8] =
    include_bytes!("../../nuklear_sys/nuklear/extra_font/Roboto-Regular.ttf");
static FONT_COUSINE: &[u8] =
    include_bytes!("../../nuklear_sys/nuklear/extra_font/Cousine-Regular.ttf");
static FONT_KARLA: &[u8] = include_bytes!("../../nuklear_sys/nuklear/extra_font/Karla-Regular.ttf");
static FONT_PROGGY: &[u8] = include_bytes!("../../nuklear_sys/nuklear/extra_font/ProggyClean.ttf");
pub(crate) static FONT_HEADING: &[u8] =
    include_bytes!("../../nuklear_sys/nuklear/extra_font/Roboto-Bold.ttf");
pub(crate) const HEADING_SIZE: f32 = 13.0;
