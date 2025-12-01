use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use once_cell::sync::Lazy;

#[derive(Default)]
pub struct Globals {
    pub target: Option<Target>,
    pub aimbot_key_down: bool,
    pub aimbot_key_editing: bool,
    pub thirdperson_pressed: bool,
    pub thirdperson_key_editing: bool,
}

#[derive(Default)]
pub struct Target {
    pub target_index: i32,
    pub should_headshot: bool,
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
