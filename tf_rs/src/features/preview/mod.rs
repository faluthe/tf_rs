//! A player drawn with the configured ESP in the menu's preview panel. The menu leaves the panel
//! see-through and the game draws the preview under it, so chams come from the real
//! DrawModelExecute hook and the overlay from the real ESP drawing.

use std::sync::{
    Mutex,
    atomic::{AtomicU8, AtomicU32, Ordering},
};

use nuklear::Rect;

use crate::{
    config::Config,
    features::{esp, theme::THEMES},
    helpers,
    interfaces::{Interfaces, Surface},
    types::{
        BBox, Player, Vec2, Vec3,
        entity::Team,
        rgba,
        view_setup::{Frustum, VIEW_CLEAR_DEPTH, ViewSetup},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    Enemy,
    Teammate,
}

impl Subject {
    pub const ALL: [Subject; 2] = [Subject::Enemy, Subject::Teammate];

    pub fn label(self) -> &'static str {
        match self {
            Subject::Enemy => "Enemy",
            Subject::Teammate => "Teammate",
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
/// Degrees the camera is turned around the model, as f32 bits
static YAW: AtomicU32 = AtomicU32::new(0);
/// Subject::ALL index plus one while the preview model is being drawn, zero otherwise
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

/// Turn the camera by a drag of `dx` pixels
pub fn rotate(dx: f32) {
    let yaw = f32::from_bits(YAW.load(Ordering::Relaxed));
    let yaw = (yaw - dx * DRAG_SPEED).rem_euclid(360.0);
    YAW.store(yaw.to_bits(), Ordering::Relaxed);
}

/// Which subject's chams the model being drawn should get, if it is the preview
pub fn drawing() -> Option<Subject> {
    match DRAWING.load(Ordering::Relaxed) {
        0 => None,
        i => Subject::ALL.get(i as usize - 1).copied(),
    }
}

/// Draw the preview where the menu put it, from the paint hook
pub fn draw(localplayer: &Player, surface: &Surface, config: &Config) {
    let Some(placement) = *PLACEMENT.lock().unwrap_or_else(|e| e.into_inner()) else {
        return;
    };
    let model = placement.model;
    if model.w < 16.0 || model.h < 16.0 {
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

    let Some(player) = model_player(localplayer) else {
        return;
    };
    let subject = subject();
    let index = player.ent.index();
    let renderable = player.ent.renderable();
    let origin = renderable.render_origin();
    let angles = renderable.render_angles();

    let mins = player.ent.mins();
    let maxs = player.ent.maxs();
    let center = Vec3::new(origin.x, origin.y, origin.z + (mins.z + maxs.z) / 2.0);
    // Wide enough for the hull turned diagonally
    let radius = mins.x.abs().max(maxs.x).max(mins.y.abs()).max(maxs.y) * 1.42;
    let yaw = angles.y + f32::from_bits(YAW.load(Ordering::Relaxed));
    let camera = Camera::new(center, yaw, (maxs.z - mins.z) / 2.0, radius, model);

    let render_view = Interfaces::render_view();
    let mut frustum: Frustum = Default::default();
    render_view.push_3d_view(&camera.view(), VIEW_CLEAR_DEPTH, &mut frustum);
    DRAWING.store(subject as u8 + 1, Ordering::Relaxed);
    Interfaces::model_render().draw_model(&renderable, index, &origin, &angles);
    DRAWING.store(0, Ordering::Relaxed);
    render_view.pop_view(&mut frustum);

    let Some(bbox) = helpers::bounding_box_with(&origin, &mins, &maxs, |p| camera.project(p))
    else {
        return;
    };
    overlay(&bbox, &player, index, localplayer, subject, surface, config);
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

/// The box, name and health bar ESP draws for `subject`
fn overlay(
    bbox: &BBox,
    player: &Player,
    index: i32,
    localplayer: &Player,
    subject: Subject,
    surface: &Surface,
    config: &Config,
) {
    if !config.esp.master {
        return;
    }
    let friendly = subject == Subject::Teammate;
    let cfg = if friendly {
        &config.esp.player_friendly
    } else {
        &config.esp.player_enemy
    };
    let team_color = match (friendly, localplayer.team()) {
        (true, team) => team.as_color(),
        (false, Team::Red) => Team::Blue.as_color(),
        (false, Team::Blue) => Team::Red.as_color(),
        (false, Team::Spectator) => &rgba::WHITE,
    };

    if cfg.boxes {
        let color = esp::esp_color(false, friendly, team_color, &config.colors.boxes);
        esp::draw_box(bbox, &color, surface);
    }
    if cfg.names {
        let color = esp::esp_color(false, friendly, team_color, &config.colors.names);
        let name = Interfaces::engine_client().get_player_info(index).name;
        let name = str::from_utf8(&name).unwrap_or("").trim_end_matches('\0');
        esp::draw_name(bbox, name, &color, surface);
    }
    if cfg.health {
        esp::draw_health(bbox, &player.ent, surface, false);
    }
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
    /// Facing `center` from the direction `yaw` points, far enough back to fit `half_height`
    /// and `radius` around it
    fn new(center: Vec3, yaw: f32, half_height: f32, radius: f32, area: Rect) -> Self {
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
        let (sin, cos) = self.yaw.to_radians().sin_cos();
        let d = Vec3::new(
            point.x - self.origin.x,
            point.y - self.origin.y,
            point.z - self.origin.z,
        );
        let forward = d.x * cos + d.y * sin;
        if forward < 1.0 {
            return None;
        }
        let right = d.x * sin - d.y * cos;
        let scale = self.area.w / 2.0 / (forward * self.tan_x);
        Some(Vec2::new(
            self.area.x + self.area.w / 2.0 + right * scale,
            self.area.y + self.area.h / 2.0 - d.z * scale,
        ))
    }
}
