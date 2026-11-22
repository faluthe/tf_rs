//! The chosen class drawn on its own when there are no players to draw, posed by the idle of
//! the class select screen and holding its stock primary

use std::{
    ptr,
    sync::{Mutex, OnceLock},
    time::Instant,
};

use nuklear::Rect;

use super::{
    Camera, Shown, Subject, chams_layers, occluded,
    pose::{self, Studio},
};
use crate::{
    config::Config,
    helpers,
    hooks::draw_model_execute::{WORLD_MODEL_SCALE, draw_layers},
    interfaces::{DrawModelInfo, Interfaces, STUDIORENDER_DRAW_NO_FLEXES, StudioRender},
    types::{
        Matrix3x4, Vec3,
        entity::Team,
        view_setup::{Frustum, VIEW_CLEAR_DEPTH},
    },
};

struct Class {
    label: &'static str,
    /// File name of the model, and of its animations with `_animations` after it
    name: &'static str,
    /// Stock primary
    weapon: &'static str,
}

/// Tried in order, the first the animations have is used
const IDLE: [&str; 3] = ["selectionmenu_idle", "stand_primary", "stand_secondary"];

const fn class(label: &'static str, name: &'static str, weapon: &'static str) -> Class {
    Class {
        label,
        name,
        weapon,
    }
}

/// Menu order
const CLASS_TABLE: [Class; 9] = [
    class("Scout", "scout", "models/weapons/c_models/c_scattergun.mdl"),
    class(
        "Soldier",
        "soldier",
        "models/weapons/c_models/c_rocketlauncher/c_rocketlauncher.mdl",
    ),
    class(
        "Pyro",
        "pyro",
        "models/weapons/c_models/c_flamethrower/c_flamethrower.mdl",
    ),
    class(
        "Demoman",
        "demo",
        "models/weapons/c_models/c_grenadelauncher/c_grenadelauncher.mdl",
    ),
    class(
        "Heavy",
        "heavy",
        "models/weapons/c_models/c_minigun/c_minigun.mdl",
    ),
    class(
        "Engineer",
        "engineer",
        "models/weapons/c_models/c_shotgun/c_shotgun.mdl",
    ),
    class(
        "Medic",
        "medic",
        "models/weapons/c_models/c_syringegun/c_syringegun.mdl",
    ),
    class(
        "Sniper",
        "sniper",
        "models/weapons/c_models/c_sniperrifle/c_sniperrifle.mdl",
    ),
    class(
        "Spy",
        "spy",
        "models/weapons/c_models/c_revolver/c_revolver.mdl",
    ),
];

pub const CLASSES: [&str; 9] = {
    let mut labels = [""; 9];
    let mut i = 0;
    while i < 9 {
        labels[i] = CLASS_TABLE[i].label;
        i += 1;
    }
    labels
};

/// Model cache handles of a class, each held once for good
#[derive(Clone, Copy)]
struct Loaded {
    model: u16,
    animations: u16,
    weapon: Option<u16>,
}

/// Indexed like CLASS_TABLE, the outer none until loading was tried
static LOADED: Mutex<[Option<Option<Loaded>>; 9]> = Mutex::new([None; 9]);

/// Standing hull of every class
const MINS: Vec3 = Vec3 {
    x: -24.0,
    y: -24.0,
    z: 0.0,
};
const MAXS: Vec3 = Vec3 {
    x: 24.0,
    y: 24.0,
    z: 82.0,
};

fn load(index: usize) -> Option<Loaded> {
    let mut loaded = LOADED.lock().unwrap_or_else(|e| e.into_inner());
    *loaded[index].get_or_insert_with(|| {
        let cache = Interfaces::mdl_cache();
        let class = &CLASS_TABLE[index];
        let find = |path: &str| cache.find_mdl(path).filter(|&h| !cache.is_error_model(h));
        let model = find(&format!("models/player/{}.mdl", class.name));
        let animations = find(&format!("models/player/{}_animations.mdl", class.name));
        let loaded = model.zip(animations).map(|(model, animations)| Loaded {
            model,
            animations,
            weapon: find(class.weapon),
        });
        if loaded.is_none() {
            log::warn!("Couldn't load the {} model for the preview", class.label);
        }
        loaded
    })
}

/// Seconds since the first class model was drawn, its animation clock
fn time() -> f32 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f32()
}

/// Draw class `index` as `subject` on `team` into `area`, the camera turned `yaw` degrees
pub(super) fn draw(
    index: usize,
    subject: Subject,
    team: &Team,
    yaw: f32,
    area: Rect,
    model_shown: bool,
    config: &Config,
) -> Option<Shown> {
    let cache = Interfaces::mdl_cache();
    let studio = Interfaces::studio_render();
    if !cache.is_valid() || !studio.is_valid() {
        return None;
    }
    let class = &CLASS_TABLE[index];
    let loaded = load(index)?;

    let origin = Vec3::default();
    // Facing +x, so turned around from there the camera sees the front
    let camera = Camera::framing(&origin, &MINS, &MAXS, yaw, area);
    let shown = Shown {
        bbox: helpers::bounding_box_with(&origin, &MINS, &MAXS, |p| camera.project(p))?,
        name: class.label.to_string(),
        health: 1,
        max_health: 1,
    };
    if !model_shown {
        return Some(shown);
    }

    // Read fresh each frame, the cache may move or drop them between frames
    let model = unsafe { Studio::from_ptr(cache.studio_hdr(loaded.model)) }?;
    let animations = unsafe { Studio::from_ptr(cache.studio_hdr(loaded.animations)) }?;
    let hardware = cache.hardware_data(loaded.model);
    if hardware.is_null() {
        return Some(shown);
    }
    let anim = IDLE.iter().find_map(|s| animations.sequence_anim(s))?;
    let bones = pose::pose(&model, &animations, &anim, time())?;
    let skin = if *team == Team::Blue { 1 } else { 0 };

    let render_view = Interfaces::render_view();
    let mut frustum: Frustum = Default::default();
    render_view.push_3d_view(&camera.view(), VIEW_CLEAR_DEPTH, &mut frustum);

    let up = Vec3::new(0.0, 0.0, 1.0);
    studio.set_view_state(&camera.origin, &camera.right(), &up, &camera.forward());
    let ambient = ambient_cube(&camera.forward());
    studio.set_ambient_light_colors(&ambient);
    studio.clear_local_lights();
    studio.set_eye_view_target(model.ptr(), 0, &camera.origin);

    let info = draw_info(model.ptr(), hardware, skin, ambient);
    if config.esp.master && config.esp.chams.enabled {
        let (layers, normal_fallback) = chams_layers(subject, &config.esp.chams);
        draw_layers(&layers, normal_fallback, WORLD_MODEL_SCALE, || {
            // The layers set the engine's color, the studio renderer keeps its own
            studio.set_color_modulation(render_view.get_color_modulation());
            studio.set_alpha_modulation(render_view.get_blend());
            draw_model(&studio, &info, &bones, &origin);
        });
    } else {
        draw_plain(&studio, &info, &bones, &origin);
    }

    // Never chams'd, so hidden with the player behind a wall
    if !occluded() {
        let weapon = loaded.weapon.and_then(|h| {
            let hdr = unsafe { Studio::from_ptr(cache.studio_hdr(h)) }?;
            let hardware = cache.hardware_data(h);
            let bones = pose::merge(&hdr, &model, &bones)?;
            (!hardware.is_null()).then(|| (draw_info(hdr.ptr(), hardware, skin, ambient), bones))
        });
        if let Some((info, bones)) = weapon {
            draw_plain(&studio, &info, &bones, &origin);
        }
    }

    render_view.pop_view(&mut frustum);
    Some(shown)
}

fn draw_info(
    studio_hdr: *const u8,
    hardware: *mut std::ffi::c_void,
    skin: i32,
    ambient: [Vec3; 6],
) -> DrawModelInfo {
    DrawModelInfo {
        studio_hdr,
        hardware_data: hardware,
        decals: ptr::null_mut(),
        skin,
        body: 0,
        hitbox_set: 0,
        client_entity: ptr::null_mut(),
        lod: 0,
        color_meshes: ptr::null_mut(),
        static_lighting: false,
        ambient_cube: ambient,
        local_light_count: 0,
        local_lights: Default::default(),
    }
}

fn draw_model(studio: &StudioRender, info: &DrawModelInfo, bones: &[Matrix3x4], origin: &Vec3) {
    studio.draw_model(info, bones, origin, STUDIORENDER_DRAW_NO_FLEXES);
}

/// With its own materials and colors
fn draw_plain(studio: &StudioRender, info: &DrawModelInfo, bones: &[Matrix3x4], origin: &Vec3) {
    studio.set_color_modulation([1.0; 3]);
    studio.set_alpha_modulation(1.0);
    draw_model(studio, info, bones, origin);
}

/// Soft light from all around, brightest from above and on the side facing the camera. Each
/// entry lights the surfaces facing +x, -x, +y, -y, +z and -z
fn ambient_cube(forward: &Vec3) -> [Vec3; 6] {
    // Surfaces facing the camera face against where it looks
    let lit = |facing: f32| {
        let v = 0.25 + 0.3 * facing.max(0.0);
        Vec3::new(v, v, v)
    };
    [
        lit(-forward.x),
        lit(forward.x),
        lit(-forward.y),
        lit(forward.y),
        Vec3::new(0.55, 0.55, 0.55),
        Vec3::new(0.12, 0.12, 0.12),
    ]
}
