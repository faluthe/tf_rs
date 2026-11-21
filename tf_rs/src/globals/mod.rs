use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use once_cell::sync::Lazy;

use crate::types::Vec3;

#[derive(Default)]
pub struct Globals {
    pub target: Option<Target>,
    pub aimbot_key_down: bool,
    pub aimbot_key_editing: bool,
    pub thirdperson_pressed: bool,
    pub thirdperson_key_editing: bool,
    pub is_spectated: bool,
    pub unload_requested: bool,
    pub prediction: Option<PredictionPaths>,
}

#[derive(Default)]
pub struct Target {
    pub target_index: i32,
    pub should_headshot: bool,
}

/// Where the projectile aimbot expected its last shot and target to go
#[derive(Default, Clone)]
pub struct PredictionPaths {
    pub player: Vec<Vec3>,
    pub projectile: Vec<Vec3>,
    /// Realtime to stop drawing at
    pub until: f32,
}

// TODO: Use arc swap?
static G: Lazy<RwLock<Globals>> = Lazy::new(|| RwLock::new(Globals::default()));

impl Globals {
    pub fn write() -> RwLockWriteGuard<'static, Globals> {
        G.write().unwrap()
    }

    pub fn read() -> RwLockReadGuard<'static, Globals> {
        G.read().unwrap()
    }
}
