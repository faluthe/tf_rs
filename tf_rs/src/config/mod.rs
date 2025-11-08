use std::sync::atomic::AtomicI32;

use once_cell::sync::Lazy;

#[derive(Default)]
pub struct Config {
    pub bunnyhop: AtomicI32,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::default());

#[macro_export]
macro_rules! cfg_enabled {
    ($field:ident) => {
        CONFIG.$field.load(std::sync::atomic::Ordering::Relaxed) != 0
    };
}
