use std::{ffi::CString, sync::OnceLock};

use log::warn;

use crate::{
    interfaces::Interfaces,
    types::{Material, material::MATERIAL_VAR_NO_DRAW},
};

pub enum Source {
    /// An existing material, by path
    Find(&'static str),
    /// A material created from VMT text
    Create(&'static str),
}

/// Menu label and where the material comes from, indexed by ChamsConfig::material
pub const MATERIALS: &[(&str, Source)] = &[
    ("Debug", Source::Find("debug/debugambientcube")),
    ("Wireframe", Source::Find("debug/debugwireframe")),
    (
        "Glow",
        Source::Create(
            r#""UnlitGeneric"
{
    "$basetexture" "vgui/white_additive"
    "$additive" "1"
    "$model" "1"
}"#,
        ),
    ),
    (
        "Hologram",
        Source::Create(
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
        ),
    ),
    (
        "Fire",
        Source::Create(
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
        ),
    ),
    (
        "Caustic",
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
        }
    }
}"#,
        ),
    ),
];

static LOADED: OnceLock<Vec<Option<Material>>> = OnceLock::new();

/// Loads or creates every chams material, call from the main thread outside of rendering
pub fn init() {
    LOADED.get_or_init(|| {
        let material_system = Interfaces::material_system();
        MATERIALS
            .iter()
            .enumerate()
            .map(|(i, (label, source))| {
                let material = match source {
                    Source::Find(path) => material_system.find_material(path),
                    Source::Create(vmt) => {
                        let name = CString::new(format!("__chams_{i}")).ok()?;
                        let vmt = CString::new(*vmt).ok()?;
                        material_system.create_material(&name, &vmt)
                    }
                };

                if material.is_none() {
                    warn!("Failed to load chams material {label}");
                }

                material.inspect(|m| m.set_material_var_flags(MATERIAL_VAR_NO_DRAW, false))
            })
            .collect()
    });
}

/// The material at `index` in MATERIALS, once init has run
pub fn material(index: usize) -> Option<Material> {
    LOADED.get()?.get(index).copied().flatten()
}
