use nuklear::{Color, Font, Nuklear, Palette};

use crate::config::Config;

pub struct Theme {
    pub name: &'static str,
    pub palette: Palette,
}

const fn c(rgb: u32) -> Color {
    Color::hex(rgb)
}

/// All flat and low saturation, they differ mostly in the one accent color.
pub const THEMES: [Theme; 6] = [
    Theme {
        // Charcoal with a brass accent
        name: "Gunmetal",
        palette: Palette {
            bg: c(0x1a1b1d),
            panel: c(0x121314),
            widget: c(0x26282b),
            widget_hover: c(0x303337),
            widget_active: c(0x2b2a26),
            border: c(0x34373b),
            text: c(0xd4d6d8),
            text_dim: c(0x8a8f96),
            accent: c(0xc9a14a),
            accent_hover: c(0xe0b95c),
        },
    },
    Theme {
        // Near black with a dried blood red
        name: "Bloodline",
        palette: Palette {
            bg: c(0x161212),
            panel: c(0x0e0b0b),
            widget: c(0x241c1c),
            widget_hover: c(0x2e2424),
            widget_active: c(0x2c1a1a),
            border: c(0x3a2b2b),
            text: c(0xddd2d0),
            text_dim: c(0x8f8280),
            accent: c(0xb3261e),
            accent_hover: c(0xd23a30),
        },
    },
    Theme {
        // Military olive drab
        name: "Olive Drab",
        palette: Palette {
            bg: c(0x1b1d17),
            panel: c(0x121410),
            widget: c(0x272b21),
            widget_hover: c(0x31362a),
            widget_active: c(0x2c3122),
            border: c(0x3a3f31),
            text: c(0xd6d8c8),
            text_dim: c(0x8c907c),
            accent: c(0x9aa850),
            accent_hover: c(0xb2c060),
        },
    },
    Theme {
        // Cold blue grey steel
        name: "Steel",
        palette: Palette {
            bg: c(0x16191d),
            panel: c(0x0e1013),
            widget: c(0x222830),
            widget_hover: c(0x2b333d),
            widget_active: c(0x1f2a36),
            border: c(0x2f3742),
            text: c(0xd3d9e0),
            text_dim: c(0x808b98),
            accent: c(0x5b9bd5),
            accent_hover: c(0x78b2e6),
        },
    },
    Theme {
        // Black and warning yellow
        name: "Hazard",
        palette: Palette {
            bg: c(0x151515),
            panel: c(0x0b0b0b),
            widget: c(0x222222),
            widget_hover: c(0x2c2c2c),
            widget_active: c(0x2a2614),
            border: c(0x343434),
            text: c(0xdedede),
            text_dim: c(0x8a8a8a),
            accent: c(0xe3b505),
            accent_hover: c(0xf5c918),
        },
    },
    Theme {
        // Pure greyscale, white is the accent
        name: "Ink",
        palette: Palette {
            bg: c(0x131313),
            panel: c(0x0a0a0a),
            widget: c(0x1f1f1f),
            widget_hover: c(0x292929),
            widget_active: c(0x303030),
            border: c(0x2e2e2e),
            text: c(0xcfcfcf),
            text_dim: c(0x808080),
            accent: c(0xf2f2f2),
            accent_hover: c(0xffffff),
        },
    },
];

pub fn current() -> &'static Theme {
    let idx = Config::read().menu.theme;
    &THEMES[idx.min(THEMES.len() - 1)]
}

/// Apply the configured theme and font. Cheap when nothing changed, so call it every frame
/// before drawing any window.
pub fn apply(nk: &Nuklear) {
    let font_idx = Config::read().menu.font;
    nk.set_palette(&current().palette);
    nk.set_font(Font::ALL[font_idx.min(Font::ALL.len() - 1)]);
}
