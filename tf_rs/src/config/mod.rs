use core::fmt;
use std::{
    env, fs,
    path::PathBuf,
    str::FromStr,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use log::error;
use once_cell::sync::Lazy;

#[derive(Default)]
pub struct Config {
    pub bunnyhop: i32, // nuklear expects i32 :P
    pub esp: ESPConfig,
    pub aimbot: AimbotConfig,
    pub thirdperson: KeyConfig,
}

#[derive(Default)]
pub struct ESPConfig {
    pub master: i32,
    pub player_friendly: i32,
    pub player_boxes: i32,
    pub player_names: i32,
    pub player_health: i32,
    pub player_conds: i32,
    pub building_friendly: i32,
    pub building_boxes: i32,
    pub building_names: i32,
    pub building_health: i32,
    pub aimbot_target: i32,
}

#[derive(Default)]
pub struct AimbotConfig {
    pub master: i32,
    pub silent_aim: i32,
    pub building_aim: i32,
    pub key: KeyConfig,
    pub fov: i32,
    pub draw_fov: i32,
}

#[derive(Default)]
pub struct KeyConfig {
    pub use_key: i32,
    pub is_mouse_button: bool,
    pub code: u32,
}

static C: Lazy<RwLock<Config>> = Lazy::new(|| {
    let cfg = Config::load_or_create("default").unwrap_or_else(|e| {
        error!("Failed to load config: {}", e);
        Config::default()
    });
    RwLock::new(cfg)
});

static L: Lazy<RwLock<Vec<String>>> = Lazy::new(|| RwLock::new(Config::get_config_names()));

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

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "bunnyhop: {}", self.bunnyhop)?;

        writeln!(f, "esp.master: {}", self.esp.master)?;
        writeln!(f, "esp.player_friendly: {}", self.esp.player_friendly)?;
        writeln!(f, "esp.player_boxes: {}", self.esp.player_boxes)?;
        writeln!(f, "esp.player_names: {}", self.esp.player_names)?;
        writeln!(f, "esp.player_health: {}", self.esp.player_health)?;
        writeln!(f, "esp.player_conds: {}", self.esp.player_conds)?;
        writeln!(f, "esp.building_friendly: {}", self.esp.building_friendly)?;
        writeln!(f, "esp.building_boxes: {}", self.esp.building_boxes)?;
        writeln!(f, "esp.building_names: {}", self.esp.building_names)?;
        writeln!(f, "esp.building_health: {}", self.esp.building_health)?;
        writeln!(f, "esp.aimbot_target: {}", self.esp.aimbot_target)?;

        writeln!(f, "aimbot.master: {}", self.aimbot.master)?;
        writeln!(f, "aimbot.silent_aim: {}", self.aimbot.silent_aim)?;
        writeln!(f, "aimbot.building_aim: {}", self.aimbot.building_aim)?;
        writeln!(f, "aimbot.key.use_key: {}", self.aimbot.key.use_key)?;
        writeln!(
            f,
            "aimbot.key.is_mouse_button: {}",
            self.aimbot.key.is_mouse_button as i32,
        )?;
        writeln!(f, "aimbot.key.code: {}", self.aimbot.key.code)?;
        writeln!(f, "aimbot.fov: {}", self.aimbot.fov)?;
        writeln!(f, "aimbot.draw_fov: {}", self.aimbot.draw_fov)?;

        writeln!(f, "thirdperson.use_key: {}", self.thirdperson.use_key)?;
        writeln!(
            f,
            "thirdperson.is_mouse_button: {}",
            self.thirdperson.is_mouse_button as i32,
        )?;
        writeln!(f, "thirdperson.code: {}", self.thirdperson.code)?;

        Ok(())
    }
}

impl FromStr for Config {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut cfg = Config::default();
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.splitn(2, ":");
            let key = parts.next().ok_or("Missing key")?.trim();
            let value_str = parts.next().ok_or("Missing value")?.trim();
            let value: i32 = value_str
                .parse()
                .map_err(|_| format!("Invalid value for {}: {}", key, value_str))?;

            match key {
                "bunnyhop" => cfg.bunnyhop = value,

                "esp.master" => cfg.esp.master = value,
                "esp.player_friendly" => cfg.esp.player_friendly = value,
                "esp.player_boxes" => cfg.esp.player_boxes = value,
                "esp.player_names" => cfg.esp.player_names = value,
                "esp.player_health" => cfg.esp.player_health = value,
                "esp.player_conds" => cfg.esp.player_conds = value,
                "esp.building_friendly" => cfg.esp.building_friendly = value,
                "esp.building_boxes" => cfg.esp.building_boxes = value,
                "esp.building_names" => cfg.esp.building_names = value,
                "esp.building_health" => cfg.esp.building_health = value,
                "esp.aimbot_target" => cfg.esp.aimbot_target = value,

                "aimbot.master" => cfg.aimbot.master = value,
                "aimbot.silent_aim" => cfg.aimbot.silent_aim = value,
                "aimbot.building_aim" => cfg.aimbot.building_aim = value,
                "aimbot.key.use_key" => cfg.aimbot.key.use_key = value,
                "aimbot.key.is_mouse_button" => cfg.aimbot.key.is_mouse_button = value != 0,
                "aimbot.key.code" => cfg.aimbot.key.code = value as u32,
                "aimbot.fov" => cfg.aimbot.fov = value,
                "aimbot.draw_fov" => cfg.aimbot.draw_fov = value,

                "thirdperson.use_key" => cfg.thirdperson.use_key = value,
                "thirdperson.is_mouse_button" => cfg.thirdperson.is_mouse_button = value != 0,
                "thirdperson.code" => cfg.thirdperson.code = value as u32,
                _ => return Err(format!("Unknown config key: {}", key)),
            }
        }
        Ok(cfg)
    }
}
