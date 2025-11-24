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
}

#[derive(Default)]
pub struct ESPConfig {
    pub master: i32,
    pub boxes: i32,
    pub names: i32,
    pub aimbot_target: i32,
    pub health: i32,
}

#[derive(Default)]
pub struct AimbotConfig {
    pub master: i32,
    pub silent_aim: i32,
    pub use_key: i32,
    pub fov: i32,
    pub draw_fov: i32,
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
        writeln!(f, "esp.boxes: {}", self.esp.boxes)?;
        writeln!(f, "esp.names: {}", self.esp.names)?;
        writeln!(f, "esp.aimbot_target: {}", self.esp.aimbot_target)?;
        writeln!(f, "esp.health: {}", self.esp.health)?;
        writeln!(f, "aimbot.master: {}", self.aimbot.master)?;
        writeln!(f, "aimbot.silent_aim: {}", self.aimbot.silent_aim)?;
        writeln!(f, "aimbot.use_key: {}", self.aimbot.use_key)?;
        writeln!(f, "aimbot.fov: {}", self.aimbot.fov)?;
        writeln!(f, "aimbot.draw_fov: {}", self.aimbot.draw_fov)?;

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
                "esp.boxes" => cfg.esp.boxes = value,
                "esp.names" => cfg.esp.names = value,
                "esp.aimbot_target" => cfg.esp.aimbot_target = value,
                "esp.health" => cfg.esp.health = value,
                "aimbot.master" => cfg.aimbot.master = value,
                "aimbot.silent_aim" => cfg.aimbot.silent_aim = value,
                "aimbot.use_key" => cfg.aimbot.use_key = value,
                "aimbot.fov" => cfg.aimbot.fov = value,
                "aimbot.draw_fov" => cfg.aimbot.draw_fov = value,
                _ => return Err(format!("Unknown config key: {}", key)),
            }
        }
        Ok(cfg)
    }
}
