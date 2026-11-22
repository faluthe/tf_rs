//! A player drawn with the configured ESP in the menu's preview panel. The menu leaves the panel
//! see-through and the game draws the preview under it. In a map a real player is drawn, so its
//! chams come from the DrawModelExecute hook. Without one a class model is posed from its own
//! animations and drawn with the same chams layers. The overlay is the real ESP drawing.

mod pose;
mod studio;

use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering},
};

use nuklear::Rect;

use crate::{
    config::{ChamsConfig, ChamsLayerConfig, Config},
    features::{esp, theme::THEMES},
    helpers,
    hooks::draw_model_execute::recolor,
    interfaces::{Interfaces, Surface},
    player_db::PlayerCategory,
    types::{
        BBox, Color, Entity, Player, Vec2, Vec3,
        entity::Team,
        rgba,
        view_setup::{Frustum, VIEW_CLEAR_DEPTH, ViewSetup},
    },
};

pub use studio::CLASSES;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    Enemy,
    Teammate,
    /// An enemy the aimbot is aiming at
    Target,
}

impl Subject {
    pub const ALL: [Subject; 3] = [Subject::Enemy, Subject::Teammate, Subject::Target];

    pub fn label(self) -> &'static str {
        match self {
            Subject::Enemy => "Enemy",
            Subject::Teammate => "Teammate",
            Subject::Target => "Target",
        }
    }
}

/// Where the menu put the preview panel this frame
#[derive(Clone, Copy)]
pub struct Placement {
    /// The whole panel, filled behind everything
    pub panel: Rect,
    /// Where the model goes
    pub model: Rect,
}

/// None while the panel isn't shown
static PLACEMENT: Mutex<Option<Placement>> = Mutex::new(None);
/// Index into Subject::ALL
static SUBJECT: AtomicU8 = AtomicU8::new(0);
/// Show the model as it looks behind a wall
static OCCLUDED: AtomicBool = AtomicBool::new(false);
/// Index into CLASSES of the class model drawn without players
static CLASS: AtomicU8 = AtomicU8::new(4);
/// Whether the last preview drawn was the class model
static CLASS_MODEL: AtomicBool = AtomicBool::new(false);
/// Degrees the camera is turned around the model, as f32 bits
static YAW: AtomicU32 = AtomicU32::new(0);
/// Subject::ALL index plus one while the preview player is being drawn, zero otherwise
static DRAWING: AtomicU8 = AtomicU8::new(0);

/// Degrees the camera turns per pixel dragged
const DRAG_SPEED: f32 = 0.6;
/// Vertical field of view of the camera
const FOV_Y: f32 = 30.0;
/// Room left around the model
const MARGIN: f32 = 1.3;

pub fn set_placement(placement: Option<Placement>) {
    *PLACEMENT.lock().unwrap_or_else(|e| e.into_inner()) = placement;
}

pub fn subject() -> Subject {
    Subject::ALL[SUBJECT.load(Ordering::Relaxed) as usize % Subject::ALL.len()]
}

pub fn set_subject(index: usize) {
    SUBJECT.store(index.min(Subject::ALL.len() - 1) as u8, Ordering::Relaxed);
}

pub fn occluded() -> bool {
    OCCLUDED.load(Ordering::Relaxed)
}

pub fn set_occluded(on: bool) {
    OCCLUDED.store(on, Ordering::Relaxed);
}

pub fn class() -> usize {
    CLASS.load(Ordering::Relaxed) as usize % CLASSES.len()
}

pub fn set_class(index: usize) {
    CLASS.store(index.min(CLASSES.len() - 1) as u8, Ordering::Relaxed);
}

/// Whether the preview shows the chosen class rather than a player
pub fn shows_class_model() -> bool {
    CLASS_MODEL.load(Ordering::Relaxed)
}

/// Turn the camera by a drag of `dx` pixels
pub fn rotate(dx: f32) {
    let yaw = f32::from_bits(YAW.load(Ordering::Relaxed));
    let yaw = (yaw - dx * DRAG_SPEED).rem_euclid(360.0);
    YAW.store(yaw.to_bits(), Ordering::Relaxed);
}

/// Which subject's chams the model being drawn should get, if it is the preview player
pub fn drawing() -> Option<Subject> {
    match DRAWING.load(Ordering::Relaxed) {
        0 => None,
        i => Subject::ALL.get(i as usize - 1).copied(),
    }
}

/// The chams layers `subject` is drawn with, and whether its own look is drawn where no layer
/// is visible. Behind a wall only the passes drawn through walls show
pub fn chams_layers(subject: Subject, config: &ChamsConfig) -> (Vec<ChamsLayerConfig>, bool) {
    let group = match subject {
        Subject::Enemy => config.enemy,
        Subject::Teammate => config.friendly,
        Subject::Target => recolor(config.enemy, &config.aimbot_target),
    };
    let layers = group.layers();
    if occluded() {
        let layers = layers
            .into_iter()
            .map(|l| ChamsLayerConfig {
                visible: false,
                ..l
            })
            .collect();
        (layers, false)
    } else {
        (layers.to_vec(), true)
    }
}

/// What the overlay is drawn around
struct Shown {
    bbox: BBox,
    name: String,
    health: i32,
    max_health: i32,
}

/// Draw the preview where the menu put it, from the paint hook. `localplayer` is none outside a
/// map
pub fn draw(localplayer: Option<&Player>, surface: &Surface, config: &Config) {
    let Some(placement) = *PLACEMENT.lock().unwrap_or_else(|e| e.into_inner()) else {
        return;
    };
    let area = placement.model;
    if area.w < 16.0 || area.h < 16.0 {
        return;
    }

    let panel = placement.panel;
    let bg = THEMES[config.menu.theme.min(THEMES.len() - 1)]
        .palette
        .panel;
    surface.draw_set_color(bg.r as i32, bg.g as i32, bg.b as i32, 255);
    surface.draw_filled_rect(
        panel.x as i32,
        panel.y as i32,
        (panel.x + panel.w) as i32,
        (panel.y + panel.h) as i32,
    );

    let subject = subject();
    let local_team = localplayer.map_or(Team::Red, |p| p.team());
    let team = subject_team(subject, &local_team);
    // Behind a wall the model only shows through chams
    let model_shown = !occluded() || (config.esp.master && config.esp.chams.enabled);
    let yaw = f32::from_bits(YAW.load(Ordering::Relaxed));

    let player = localplayer.and_then(model_player);
    CLASS_MODEL.store(player.is_none(), Ordering::Relaxed);
    let shown = match player {
        Some(player) => draw_player(&player, subject, yaw, area, model_shown),
        None => studio::draw(class(), subject, &team, yaw, area, model_shown, config),
    };
    if let Some(shown) = shown {
        overlay(&shown, subject, &team, surface, config);
    }
}

/// The team `subject` is on when the local player is on `local`
fn subject_team(subject: Subject, local: &Team) -> Team {
    let friendly = subject == Subject::Teammate;
    match (friendly, local) {
        (true, Team::Blue) | (false, Team::Red) => Team::Blue,
        _ => Team::Red,
    }
}

/// The local player while alive, else anyone who is
fn model_player(localplayer: &Player) -> Option<Player> {
    if !localplayer.is_dead() {
        return Some(Player {
            ent: localplayer.ent,
        });
    }
    let entity_list = Interfaces::entity_list();
    (1..=Interfaces::engine_client().get_max_clients())
        .filter_map(|i| entity_list.get_client_entity::<Player>(i))
        .find(|p| !p.is_dormant() && !p.is_dead())
}

/// Draw `player` and what it wears and holds as `subject`
fn draw_player(
    player: &Player,
    subject: Subject,
    yaw: f32,
    area: Rect,
    model_shown: bool,
) -> Option<Shown> {
    let index = player.ent.index();
    let renderable = player.ent.renderable();
    let origin = renderable.render_origin();
    let angles = renderable.render_angles();
    let mins = player.ent.mins();
    let maxs = player.ent.maxs();
    let camera = Camera::framing(&origin, &mins, &maxs, angles.y + yaw, area);

    let render_view = Interfaces::render_view();
    let model_render = Interfaces::model_render();
    let mut frustum: Frustum = Default::default();
    render_view.push_3d_view(&camera.view(), VIEW_CLEAR_DEPTH, &mut frustum);
    if model_shown {
        DRAWING.store(subject as u8 + 1, Ordering::Relaxed);
        model_render.draw_model(&renderable, index, &origin, &angles);
        DRAWING.store(0, Ordering::Relaxed);
        // Never chams'd, so hidden with the player behind a wall
        if !occluded() {
            for attachment in attachments(player) {
                let renderable = attachment.renderable();
                model_render.draw_model(
                    &renderable,
                    attachment.index(),
                    &renderable.render_origin(),
                    &renderable.render_angles(),
                );
            }
        }
    }
    render_view.pop_view(&mut frustum);

    let name = Interfaces::engine_client().get_player_info(index).name;
    let name = str::from_utf8(&name).unwrap_or("").trim_end_matches('\0');
    Some(Shown {
        bbox: helpers::bounding_box_with(&origin, &mins, &maxs, |p| camera.project(p))?,
        name: name.to_string(),
        health: player.ent.health(),
        max_health: player.ent.max_health(),
    })
}

/// Cosmetics and the held weapon of `player`
fn attachments(player: &Player) -> Vec<Entity> {
    let weapon = player.active_weapon_index();
    let mut out = Vec::new();
    let mut child = player.ent.renderable().first_shadow_child();
    // Bounded in case the list is caught changing
    for _ in 0..64 {
        let Some(renderable) = child else {
            break;
        };
        let entity = renderable.entity();
        let class = entity.class_name();
        if class.starts_with("CTFWearable")
            || class == "CTFPowerupBottle"
            || entity.index() == weapon
        {
            out.push(entity);
        }
        child = renderable.next_shadow_peer();
    }
    out
}

/// The box, name, health bar and labels ESP draws for `subject` on `team`
fn overlay(shown: &Shown, subject: Subject, team: &Team, surface: &Surface, config: &Config) {
    if !config.esp.master {
        return;
    }
    let friendly = subject == Subject::Teammate;
    let cfg = if friendly {
        &config.esp.player_friendly
    } else {
        &config.esp.player_enemy
    };
    if friendly && !cfg.bool() {
        return;
    }
    let is_target = subject == Subject::Target && config.esp.aimbot_target;
    let team_color = team.as_color();
    let bbox = &shown.bbox;

    if cfg.boxes {
        let color = esp::esp_color(is_target, friendly, team_color, &config.colors.boxes);
        esp::draw_box(bbox, &color, surface);
    }
    if cfg.names {
        let color = esp::esp_color(is_target, friendly, team_color, &config.colors.names);
        esp::draw_name(bbox, &shown.name, &color, surface);
    }
    if cfg.health {
        esp::draw_health_values(bbox, shown.health, shown.max_health, surface, false);
    }
    esp::draw_conds(bbox, &labels(subject, config), surface);
}

/// Every label ESP could put next to `subject`, whether or not it applies right now
fn labels(subject: Subject, config: &Config) -> Vec<(String, Color)> {
    let mut labels = Vec::new();
    if subject == Subject::Target && config.esp.aimbot_target {
        labels.push(("TARGET".to_string(), rgba::WHITE));
        labels.push(("HS".to_string(), rgba::RED));
    }
    let cc = &config.esp.conds;
    let enemy_only = subject != Subject::Teammate;
    for (cond, name, shown) in [
        (&cc.disguised, "DISGUISED", true),
        (&cc.taunting, "TAUNTING", true),
        (&cc.zoomed, "ZOOMED", true),
        (&cc.invisible, "INVISIBLE", true),
        (&cc.milked, "MILKED", true),
        (&cc.mg, "NO MG", enemy_only),
        (&cc.butter, "BUTTER", enemy_only),
    ] {
        if shown && cond.enabled {
            labels.push((name.to_string(), cond.color));
        }
    }
    if cc.category {
        let category = PlayerCategory::Category1;
        labels.push((
            category.display_name(),
            category.color(&config.colors.player_categories),
        ));
    }
    labels
}

/// Level camera looking at a point from `distance` away, drawing into `area`
struct Camera {
    origin: Vec3,
    yaw: f32,
    /// tan of half the horizontal field of view
    tan_x: f32,
    area: Rect,
}

impl Camera {
    /// Looking at a hull from `mins` to `maxs` at `origin` from the direction `yaw` points, far
    /// enough back to fit it turned any way
    fn framing(origin: &Vec3, mins: &Vec3, maxs: &Vec3, yaw: f32, area: Rect) -> Self {
        let center = Vec3::new(origin.x, origin.y, origin.z + (mins.z + maxs.z) / 2.0);
        // Wide enough for the hull turned diagonally
        let radius = mins.x.abs().max(maxs.x).max(mins.y.abs()).max(maxs.y) * 1.42;
        let half_height = (maxs.z - mins.z) / 2.0;

        let aspect = area.w / area.h;
        let tan_y = (FOV_Y / 2.0).to_radians().tan();
        let tan_x = tan_y * aspect;
        let distance = (half_height / tan_y).max(radius / tan_x) * MARGIN + radius;
        let (sin, cos) = yaw.to_radians().sin_cos();
        Camera {
            origin: Vec3::new(
                center.x + cos * distance,
                center.y + sin * distance,
                center.z,
            ),
            yaw: yaw + 180.0,
            tan_x,
            area,
        }
    }

    fn forward(&self) -> Vec3 {
        let (sin, cos) = self.yaw.to_radians().sin_cos();
        Vec3::new(cos, sin, 0.0)
    }

    fn right(&self) -> Vec3 {
        let (sin, cos) = self.yaw.to_radians().sin_cos();
        Vec3::new(sin, -cos, 0.0)
    }

    fn view(&self) -> ViewSetup {
        let (x, y) = (self.area.x as i32, self.area.y as i32);
        let (w, h) = (self.area.w as i32, self.area.h as i32);
        let fov = (self.tan_x.atan() * 2.0).to_degrees();
        ViewSetup {
            x,
            unscaled_x: x,
            y,
            unscaled_y: y,
            width: w,
            unscaled_width: w,
            height: h,
            unscaled_height: h,
            fov,
            fov_viewmodel: fov,
            origin: self.origin,
            angles: Vec3::new(0.0, self.yaw, 0.0),
            z_near: 4.0,
            z_far: 4096.0,
            z_near_viewmodel: 4.0,
            z_far_viewmodel: 4096.0,
            aspect_ratio: self.area.w / self.area.h,
            ..Default::default()
        }
    }

    /// Where `point` lands on screen, none behind the camera
    fn project(&self, point: &Vec3) -> Option<Vec2> {
        let d = Vec3::new(
            point.x - self.origin.x,
            point.y - self.origin.y,
            point.z - self.origin.z,
        );
        let (f, r) = (self.forward(), self.right());
        let forward = d.x * f.x + d.y * f.y;
        if forward < 1.0 {
            return None;
        }
        let right = d.x * r.x + d.y * r.y;
        let scale = self.area.w / 2.0 / (forward * self.tan_x);
        Some(Vec2::new(
            self.area.x + self.area.w / 2.0 + right * scale,
            self.area.y + self.area.h / 2.0 - d.z * scale,
        ))
    }
}
