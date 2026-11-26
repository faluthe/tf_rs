//! Projecting world positions onto the screen from the main view.
//!
//! The engine's own projection (IVDebugOverlay::ScreenPosition) uses the matrix of the last view it
//! rendered. Maps that render more views after the main one, like water reflections or camera
//! screens, leave it pointing somewhere else. The main view's camera is recorded each frame
//! instead and projected the way the engine sets it up.

use std::sync::Mutex;

use crate::{
    interfaces::Interfaces,
    types::{Vec2, Vec3, ViewSetup},
};

#[derive(Clone, Copy)]
struct Camera {
    origin: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    /// Horizontal field of view in degrees, for a 4:3 screen like the fov setting
    fov: f32,
}

static CAMERA: Mutex<Option<Camera>> = Mutex::new(None);

/// Remember the main view's camera, from OverrideView once it's final
pub fn record(setup: &ViewSetup) {
    let (forward, right, up) = setup.angles.angle_vectors();
    *CAMERA.lock().unwrap() = Some(Camera {
        origin: setup.origin,
        forward,
        right,
        up,
        fov: setup.fov,
    });
}

/// Where `point` is on the screen, none when it's behind the camera
pub fn world_to_screen(point: &Vec3) -> Option<Vec2> {
    let Some(camera) = *CAMERA.lock().unwrap() else {
        return Interfaces::debug_overlay().screen_position(point);
    };
    let (width, height) = Interfaces::engine_client().get_screen_size();
    let (width, height) = (width as f32, height as f32);
    let delta = *point - camera.origin;
    let depth = delta.dot(&camera.forward);
    if depth < 0.1 {
        return None;
    }
    // The fov is for 4:3, the engine widens it to the screen's aspect ratio. Its half width at
    // depth 1 is then tan(fov / 2) * aspect * 3 / 4, which spans half the screen width.
    let half = (camera.fov / 2.0).to_radians().tan() * (width / height) * 0.75;
    let scale = (width / 2.0) / half;
    Some(Vec2::new(
        width / 2.0 + delta.dot(&camera.right) / depth * scale,
        height / 2.0 - delta.dot(&camera.up) / depth * scale,
    ))
}
