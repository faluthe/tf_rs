use std::{
    env, fs,
    path::PathBuf,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use log::error;
use once_cell::sync::Lazy;

use crate::struct_with_serialize;

static C: Lazy<RwLock<Config>> = Lazy::new(|| {
    let cfg = Config::load_or_create("default").unwrap_or_else(|e| {
        error!("Failed to load config: {}", e);
        Config::default()
    });
    RwLock::new(cfg)
});

static L: Lazy<RwLock<Vec<String>>> = Lazy::new(|| RwLock::new(Config::get_config_names()));

struct_with_serialize! {
    #[derive(Default)]
    pub struct Config {
        pub bunnyhop: bool,
        pub esp: ESPConfig,
        pub aimbot: AimbotConfig,
        pub thirdperson: KeyConfig,
        pub spectator_list: bool,
    }
}

struct_with_serialize! {
    #[derive(Default)]
    pub struct ESPConfig {
        pub master: bool,
        pub player_enemy: EntityESPConfig,
        pub player_friendly: EntityESPConfig,
        pub building_enemy: EntityESPConfig,
        pub building_friendly: EntityESPConfig,
        pub aimbot_target: bool,
    }
}

struct_with_serialize! {
    #[derive(Default)]
    pub struct EntityESPConfig {
        pub boxes: bool,
        pub names: bool,
        pub health: bool,
        pub conds: bool,
    }
}

struct_with_serialize! {
    #[derive(Default)]
    pub struct AimbotConfig {
        pub master: bool,
        pub silent_aim: bool,
        pub building_aim: bool,
        pub key: KeyConfig,
        pub projectile: ProjectileAimbotConfig,
        pub fov: i32,
        pub draw_fov: bool,
    }
}

struct_with_serialize! {
    #[derive(Default)]
    pub struct ProjectileAimbotConfig {
        pub step_time: f32,
        pub max_steps: i32,
        pub tolerance: f32,
    }
}

struct_with_serialize! {
    #[derive(Default)]
    pub struct KeyConfig {
        pub use_key: bool,
        pub is_mouse_button: bool,
        pub code: u32,
    }
}

impl Config {
    pub fn write() -> RwLockWriteGuard<'static, Config> {
        C.write().unwrap()
    }

    pub fn read() -> RwLockReadGuard<'static, Config> {
        C.read().unwrap()
    }

    pub fn list_configs() -> RwLockReadGuard<'static, Vec<String>> {
        L.read().unwrap()
    }

    pub fn refresh_configs() {
        let mut w = L.write().unwrap();
        w.clear();
        w.extend(Config::get_config_names());
    }

    // TODO: Clean this up
    fn get_config_names() -> Vec<String> {
        let home = match env::var("HOME") {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };
        let mut configs = Vec::new();
        let dir = match fs::read_dir(&home) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };
        for entry in dir {
            if let Ok(entry) = entry {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                if file_name.starts_with('.') && file_name.ends_with(".tf_rs.cfg") {
                    configs.push(file_name.to_string());
                }
            }
        }

        configs
            .iter()
            .map(|s| {
                s.trim_start_matches('.')
                    .trim_end_matches(".tf_rs.cfg")
                    .to_string()
            })
            .collect()
    }

    pub fn load(&mut self, name: &str) -> anyhow::Result<()> {
        let path = Config::get_path(name)?;

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(s) => {
                    if let Ok(cfg) = s.parse::<Config>() {
                        *self = cfg;
                        return Ok(());
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to read config file: {}", e));
                }
            }
        }

        Err(anyhow::anyhow!("Config file does not exist"))
    }

    pub fn save(&self, name: &str) -> anyhow::Result<()> {
        let path = Config::get_path(name)?;

        fs::write(&path, self.to_string())?;

        Ok(())
    }

    fn load_or_create(name: &str) -> anyhow::Result<Self> {
        let path = Config::get_path(name)?;

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(s) => {
                    if let Ok(cfg) = s.parse::<Config>() {
                        return Ok(cfg);
                    }
                }
                Err(_) => {}
            }
        }

        let cfg = Config::default();

        fs::write(&path, cfg.to_string())?;

        Ok(cfg)
    }

    fn get_path(name: &str) -> anyhow::Result<PathBuf> {
        let home = env::var("HOME")?;

        let mut path = PathBuf::from(home);
        path.push(format!(".{}.tf_rs.cfg", name));

        Ok(path)
    }
}

impl EntityESPConfig {
    pub fn bool(&self) -> bool {
        self.boxes || self.names || self.health || self.conds
    }
}
