use std::{
    ffi::{CStr, CString},
    sync::OnceLock,
};

use log::warn;

use crate::{
    interfaces::Interfaces,
    types::{Color, Material, material::MATERIAL_VAR_NO_DRAW},
};

/// A loaded layer. Its parameters are looked up on every use, a map change rebuilds them
pub struct Loaded {
    pub material: Material,
}

/// Parameters the draw color doesn't reach, set to it instead
const TINTS: [&CStr; 3] = [c"$phongtint", c"$envmaptint", c"$selfillumtint"];

impl Loaded {
    /// Texture scale of materials that read it from `$chamsscale`
    pub fn scale(&self, scale: f32) {
        if let Some(var) = self.material.find_var(c"$chamsscale") {
            var.set_float(scale);
        }
    }

    /// Tint reflections and self illumination `color`
    pub fn tint(&self, color: Color) {
        for var in TINTS.iter().filter_map(|name| self.material.find_var(name)) {
            var.set_vec3(
                color.r as f32 / 255.0,
                color.g as f32 / 255.0,
                color.b as f32 / 255.0,
            );
        }
    }
}

pub struct ChamsMaterial {
    pub label: &'static str,
    pub layers: &'static [Layer],
}

/// One draw of a chams material, in a shade of the configured color that drifts over time
pub struct Layer {
    pub source: Source,
    /// Degrees the hue swings either side of the configured color
    pub hue_swing: f32,
    /// How fast the hue swings, in radians per second
    pub speed: f32,
    /// Where in the swing this layer starts, so layers drift apart
    pub phase: f32,
    /// Brightness relative to the configured color
    pub value: f32,
    /// Texture scale for materials that read it from `$chamsscale`, before the model's own
    pub scale: f32,
}

impl Layer {
    /// Exactly the configured color
    const fn plain(source: Source) -> Self {
        Layer {
            source,
            hue_swing: 0.0,
            speed: 0.0,
            phase: 0.0,
            value: 1.0,
            scale: 1.0,
        }
    }
}

pub enum Source {
    /// An existing material, by path
    Find(&'static str),
    /// A material created from VMT text
    Create(&'static str),
}

/// Menu label and the layers drawn on top of each other, indexed by ChamsLayerConfig::material.
/// New ones go at the end so saved indexes keep pointing at the same material
pub const MATERIALS: &[ChamsMaterial] = &[
    ChamsMaterial {
        label: "Debug",
        layers: &[Layer::plain(Source::Find("debug/debugambientcube"))],
    },
    ChamsMaterial {
        label: "Wireframe",
        layers: &[Layer::plain(Source::Find("debug/debugwireframe"))],
    },
    ChamsMaterial {
        label: "Glow",
        layers: &[Layer::plain(Source::Create(
            r#""UnlitGeneric"
{
    "$basetexture" "vgui/white_additive"
    "$additive" "1"
    "$model" "1"
}"#,
        ))],
    },
    ChamsMaterial {
        label: "Hologram",
        layers: &[Layer::plain(Source::Create(
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
        ))],
    },
    ChamsMaterial {
        label: "Fire",
        layers: &[Layer::plain(Source::Create(
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
        ))],
    },
    ChamsMaterial {
        label: "Caustic",
        layers: &[
            // Solid, lit base for the caustics to sit on
            Layer {
                source: Source::Create(
                    r#""VertexLitGeneric"
{
    "$basetexture" "vgui/white"
}"#,
                ),
                hue_swing: 25.0,
                speed: 0.5,
                phase: 0.0,
                value: 0.45,
                scale: 1.0,
            },
            Layer {
                source: Source::Create(
                    r#""UnlitGeneric"
{
    "$basetexture" "effects/animatedcaustic/caustic"
    "$chamsscale" "1"
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
            "texturescale" "$chamsscale"
        }
    }
}"#,
                ),
                hue_swing: 60.0,
                speed: 0.9,
                phase: 0.0,
                value: 1.0,
                scale: 1.0,
            },
            Layer {
                source: Source::Create(
                    r#""UnlitGeneric"
{
    "$basetexture" "effects/animatedcaustic/caustic"
    "$chamsscale" "1"
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
            "texturescale" "$chamsscale"
        }
    }
}"#,
                ),
                hue_swing: 60.0,
                speed: 0.7,
                phase: 2.1,
                value: 0.85,
                scale: 1.7,
            },
            Layer {
                source: Source::Create(
                    r#""UnlitGeneric"
{
    "$basetexture" "effects/animatedcaustic/caustic"
    "$chamsscale" "1"
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
            "texturescale" "$chamsscale"
        }
    }
}"#,
                ),
                hue_swing: 60.0,
                speed: 1.2,
                phase: 4.2,
                value: 1.0,
                scale: 0.6,
            },
        ],
    },
    // The model's own materials, nothing to load
    ChamsMaterial {
        label: "Original",
        layers: &[],
    },
    // Glowing edges and a see-through middle, the sky reflected at grazing angles
    ChamsMaterial {
        label: "Fresnel",
        layers: &[Layer::plain(Source::Create(
            r#""VertexLitGeneric"
{
    "$basetexture" "white"
    "$bumpmap" "models/player/shared/shared_normal"
    "$color2" "[0 0 0]"
    "$additive" "1"
    "$phong" "1"
    "$phongfresnelranges" "[0 0.5 1]"
    "$envmap" "skybox/sky_dustbowl_01"
    "$envmapfresnel" "1"
    "$model" "1"
}"#,
        ))],
    },
    // Glossy sheen
    ChamsMaterial {
        label: "Shine",
        layers: &[Layer::plain(Source::Create(
            r#""VertexLitGeneric"
{
    "$additive" "1"
    "$envmap" "cubemaps/cubemap_sheen002.hdr"
    "$envmaptint" "[1 1 1]"
    "$model" "1"
}"#,
        ))],
    },
    // Ice statue lit in the layer's color
    ChamsMaterial {
        label: "Tint",
        layers: &[Layer::plain(Source::Create(
            r#""VertexLitGeneric"
{
    "$basetexture" "models/player/shared/ice_player"
    "$bumpmap" "models/player/shared/shared_normal"
    "$additive" "1"
    "$phong" "1"
    "$phongfresnelranges" "[0 0.001 0.001]"
    "$envmap" "skybox/sky_dustbowl_01"
    "$envmapfresnel" "1"
    "$selfillum" "1"
    "$selfillumtint" "[0 0 0]"
    "$model" "1"
}"#,
        ))],
    },
];

/// Whether the material at `index` draws the model with its own materials
pub fn is_original(index: usize) -> bool {
    MATERIALS.get(index).is_some_and(|m| m.layers.is_empty())
}

/// Each material's layers, empty if any of them failed to load
static LOADED: OnceLock<Vec<Vec<Loaded>>> = OnceLock::new();

/// Loads or creates every chams material, call from the main thread outside of rendering
pub fn init() {
    LOADED.get_or_init(|| {
        let material_system = Interfaces::material_system();
        MATERIALS
            .iter()
            .enumerate()
            .map(|(i, chams_material)| {
                let loaded: Option<Vec<Material>> = chams_material
                    .layers
                    .iter()
                    .enumerate()
                    .map(|(layer, Layer { source, .. })| match source {
                        Source::Find(path) => material_system.find_material(path),
                        Source::Create(vmt) => {
                            let name = CString::new(format!("__chams_{i}_{layer}")).ok()?;
                            let vmt = CString::new(*vmt).ok()?;
                            material_system.create_material(&name, &vmt)
                        }
                    })
                    .collect();

                let Some(loaded) = loaded else {
                    warn!("Failed to load chams material {}", chams_material.label);
                    return Vec::new();
                };

                loaded
                    .into_iter()
                    .map(|material| {
                        material.set_material_var_flags(MATERIAL_VAR_NO_DRAW, false);
                        Loaded { material }
                    })
                    .collect()
            })
            .collect()
    });
}

/// Layers of the material at `index` in MATERIALS and their loaded materials, once init has run
pub fn layers(index: usize) -> Option<(&'static [Layer], &'static [Loaded])> {
    let loaded = LOADED.get()?.get(index)?;
    (!loaded.is_empty()).then(|| (MATERIALS[index].layers, loaded.as_slice()))
}

impl Layer {
    /// This layer's shade of `base` at `time` seconds
    pub fn color(&self, base: Color, time: f32) -> Color {
        if self.hue_swing == 0.0 && self.value == 1.0 {
            return base;
        }

        let (h, s, v) = rgb_to_hsv(base);
        let h = h + self.hue_swing * (time * self.speed + self.phase).sin();
        let (r, g, b) = hsv_to_rgb(h.rem_euclid(360.0), s, (v * self.value).min(1.0));
        Color { r, g, b, a: base.a }
    }
}

fn rgb_to_hsv(c: Color) -> (f32, f32, f32) {
    let (r, g, b) = (c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0);
    let max = r.max(g).max(b);
    let delta = max - r.min(g).min(b);
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { delta / max };
    (h, s, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let q = |f: f32| ((f + m) * 255.0).round() as u8;
    (q(r), q(g), q(b))
}
