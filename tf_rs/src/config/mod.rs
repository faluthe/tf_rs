use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use once_cell::sync::Lazy;

#[derive(Default)]
pub struct Config {
    pub bunnyhop: i32, // nuklear expects i32 :P
    pub esp: ESPConfig,
    pub aimbot: AimbotConfig,
}

#[derive(Default)]
pub struct ESPConfig {
    pub master: i32,
    pub boxes: i32,
    pub names: i32,
    pub aimbot_target: i32,
}

#[derive(Default)]
pub struct AimbotConfig {
    pub master: i32,
    pub silent_aim: i32,
    pub use_key: i32,
    pub fov: i32,
    pub draw_fov: i32,
}

static C: Lazy<RwLock<Config>> = Lazy::new(|| RwLock::new(Config::default()));

impl Config {
    pub fn write() -> RwLockWriteGuard<'static, Config> {
        C.write().unwrap()
    }

    pub fn read() -> RwLockReadGuard<'static, Config> {
        C.read().unwrap()
    }
}
