use std::sync::atomic::AtomicI32;

use once_cell::sync::Lazy;

#[derive(Default)]
pub struct Config {
    pub bunnyhop: AtomicI32,
    pub esp: AtomicI32,
    pub esp_boxes: AtomicI32,
    pub esp_names: AtomicI32,
    pub esp_aimbot_target: AtomicI32,
    pub aimbot: AtomicI32,
    pub silent_aim: AtomicI32,
    pub aimbot_fov: AtomicI32,
    pub draw_fov: AtomicI32,
    pub use_aimbot_key: AtomicI32,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::default());

#[macro_export]
macro_rules! cfg_enabled {
    ($field:ident) => {
        crate::config::CONFIG
            .$field
            .load(std::sync::atomic::Ordering::Relaxed)
            != 0
    };
}

#[macro_export]
macro_rules! cfg_get {
    ($field:ident) => {
        crate::config::CONFIG
            .$field
            .load(std::sync::atomic::Ordering::Relaxed)
    };
}
