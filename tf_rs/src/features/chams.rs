use std::{ffi::CString, sync::OnceLock};

use log::warn;

use crate::{
    interfaces::Interfaces,
    types::{Color, Material, material::MATERIAL_VAR_NO_DRAW},
};

pub enum Source {
    /// An existing material, by path
    Find(&'static str),
    /// A material created from VMT text
    Create(&'static str),
}

/// Menu label and the layers drawn on top of each other, indexed by ChamsGroupConfig::material
pub const MATERIALS: &[(&str, &[Source])] = &[
    ("Debug", &[Source::Find("debug/debugambientcube")]),
    ("Wireframe", &[Source::Find("debug/debugwireframe")]),
    (
        "Glow",
        &[Source::Create(
            r#""UnlitGeneric"
{
    "$basetexture" "vgui/white_additive"
    "$additive" "1"
    "$model" "1"
}"#,
        )],
    ),
    (
        "Hologram",
        &[Source::Create(
            r#""UnlitTwoTexture"
{
    "$basetexture" "vgui/white_additive"
    "$texture2" "models/effects/cappoint_beam_lines_b"
    "$additive" "1"
    "$model" "1"
    "Proxies"
    {
        "TextureScroll"
        {
            "texturescrollvar" "$texture2transform"
            "texturescrollrate" ".25"
            "texturescrollangle" "90"
        }
    }
}"#,
        )],
    ),
    (
        "Fire",
        &[Source::Create(
            r#""UnlitGeneric"
{
    "$basetexture" "effects/tiledfire/firelayeredslowtiled512"
    "$additive" "1"
    "$model" "1"
    "Proxies"
    {
        "AnimatedTexture"
        {
            "animatedtexturevar" "$basetexture"
            "animatedtextureframenumvar" "$frame"
            "animatedtextureframerate" "30"
        }
    }
}"#,
        )],
    ),
    (
        "Caustic",
        &[
            Source::Create(
                r#""UnlitGeneric"
{
    "$basetexture" "effects/animatedcaustic/caustic"
    "$additive" "1"
    "$model" "1"
    "Proxies"
    {
        "AnimatedTexture"
        {
            "animatedtexturevar" "$basetexture"
            "animatedtextureframenumvar" "$frame"
            "animatedtextureframerate" "24"
        }
        "TextureScroll"
        {
            "texturescrollvar" "$basetexturetransform"
            "texturescrollrate" ".15"
            "texturescrollangle" "60"
            "texturescale" "1"
        }
    }
}"#,
            ),
            Source::Create(
                r#""UnlitGeneric"
{
    "$basetexture" "effects/animatedcaustic/caustic"
    "$additive" "1"
    "$model" "1"
    "Proxies"
    {
        "AnimatedTexture"
        {
            "animatedtexturevar" "$basetexture"
            "animatedtextureframenumvar" "$frame"
            "animatedtextureframerate" "17"
        }
        "TextureScroll"
        {
            "texturescrollvar" "$basetexturetransform"
            "texturescrollrate" ".1"
            "texturescrollangle" "200"
            "texturescale" "1.7"
        }
    }
}"#,
            ),
            Source::Create(
                r#""UnlitGeneric"
{
    "$basetexture" "effects/animatedcaustic/caustic"
    "$additive" "1"
    "$model" "1"
    "Proxies"
    {
        "AnimatedTexture"
        {
            "animatedtexturevar" "$basetexture"
            "animatedtextureframenumvar" "$frame"
            "animatedtextureframerate" "11"
        }
        "TextureScroll"
        {
            "texturescrollvar" "$basetexturetransform"
            "texturescrollrate" ".06"
            "texturescrollangle" "320"
            "texturescale" ".6"
        }
    }
}"#,
            ),
        ],
    ),
];

// Each material's layers, empty if any of them failed to load
static LOADED: OnceLock<Vec<Vec<Material>>> = OnceLock::new();

/// Loads or creates every chams material, call from the main thread outside of rendering
pub fn init() {
    LOADED.get_or_init(|| {
        let material_system = Interfaces::material_system();
        MATERIALS
            .iter()
            .enumerate()
            .map(|(i, (label, layers))| {
                let loaded: Option<Vec<Material>> = layers
                    .iter()
                    .enumerate()
                    .map(|(layer, source)| match source {
                        Source::Find(path) => material_system.find_material(path),
                        Source::Create(vmt) => {
                            let name = CString::new(format!("__chams_{i}_{layer}")).ok()?;
                            let vmt = CString::new(*vmt).ok()?;
                            material_system.create_material(&name, &vmt)
                        }
                    })
                    .collect();

                let Some(loaded) = loaded else {
                    warn!("Failed to load chams material {label}");
                    return Vec::new();
                };

                for material in &loaded {
                    material.set_material_var_flags(MATERIAL_VAR_NO_DRAW, false);
                }
                loaded
            })
            .collect()
    });
}

/// Layers of the material at `index` in MATERIALS, once init has run
pub fn layers(index: usize) -> Option<&'static [Material]> {
    let layers = LOADED.get()?.get(index)?;
    (!layers.is_empty()).then_some(layers.as_slice())
}

/// Shade of `base` for a layer, the first layer is `base` itself and later ones alternate
/// between darker and lighter
pub fn layer_color(base: Color, layer: usize) -> Color {
    let darken = |c: u8| (c as f32 * 0.6) as u8;
    let lighten = |c: u8| (c as f32 + (255.0 - c as f32) * 0.4) as u8;
    let shade: &dyn Fn(u8) -> u8 = match layer {
        0 => return base,
        l if l % 2 == 1 => &darken,
        _ => &lighten,
    };
    Color {
        r: shade(base.r),
        g: shade(base.g),
        b: shade(base.b),
        a: base.a,
    }
}
