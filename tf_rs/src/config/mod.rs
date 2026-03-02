use std::{
    env, fs,
    path::PathBuf,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use log::error;
use once_cell::sync::Lazy;

use crate::{struct_with_serialize, types::rgba::ColorF};

static C: Lazy<RwLock<Config>> = Lazy::new(|| {
    let cfg = Config::load_or_create("default").unwrap_or_else(|e| {
        error!("Failed to load config: {}", e);
        Config::default()
    });
    RwLock::new(cfg)
});

static L: Lazy<RwLock<Vec<String>>> = Lazy::new(|| {
    let mut names = Config::get_config_names();
    if let Some(pos) = names.iter().position(|s| s == "default") {
        names.swap(0, pos);
    }
    RwLock::new(names)
});

struct_with_serialize! {
    #[derive(Default)]
    pub struct Config {
        pub bunnyhop: bool,
        pub esp: ESPConfig,
        pub aimbot: AimbotConfig,
        pub thirdperson: KeyConfig,
        pub spectator_list: bool,
        pub colors: ColorsConfig,
    }
}

struct_with_serialize! {
    pub struct EspColorConfig {
        pub use_team_colors: bool,
        pub enemy: ColorF,
        pub friendly: ColorF,
    }
}

impl Default for EspColorConfig {
    fn default() -> Self {
        EspColorConfig {
            use_team_colors: true,
            // Match rgba::RED (220, 45, 35)
            enemy: ColorF {
                r: 0.863,
                g: 0.176,
                b: 0.137,
                a: 1.0,
            },
            // Match rgba::BLUE (40, 110, 240)
            friendly: ColorF {
                r: 0.157,
                g: 0.431,
                b: 0.941,
                a: 1.0,
            },
        }
    }
}

struct_with_serialize! {
    pub struct BuildingColorConfig {
        pub enemy: ColorF,
        pub friendly: ColorF,
    }
}

impl Default for BuildingColorConfig {
    fn default() -> Self {
        BuildingColorConfig {
            enemy: ColorF {
                r: 0.863,
                g: 0.176,
                b: 0.137,
                a: 1.0,
            },
            friendly: ColorF {
                r: 0.157,
                g: 0.431,
                b: 0.941,
                a: 1.0,
            },
        }
    }
}

struct_with_serialize! {
    pub struct BuildingsColorsConfig {
        pub use_team_colors: bool,
        pub sentry: BuildingColorConfig,
        pub dispenser: BuildingColorConfig,
        pub teleporter: BuildingColorConfig,
    }
}

impl Default for BuildingsColorsConfig {
    fn default() -> Self {
        BuildingsColorsConfig {
            use_team_colors: true,
            sentry: BuildingColorConfig::default(),
            dispenser: BuildingColorConfig::default(),
            teleporter: BuildingColorConfig::default(),
        }
    }
}

struct_with_serialize! {
    pub struct ColorsConfig {
        pub boxes: EspColorConfig,
        pub names: EspColorConfig,
        pub buildings: BuildingsColorsConfig,
    }
}

impl Default for ColorsConfig {
    fn default() -> Self {
        ColorsConfig {
            boxes: EspColorConfig::default(),
            names: EspColorConfig::default(),
            buildings: BuildingsColorsConfig::default(),
        }
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
        pub conds: CondsDisplayConfig,
    }
}

struct_with_serialize! {
    pub struct CondDisplayConfig {
        pub enabled: bool,
        pub color: ColorF,
    }
}

impl Default for CondDisplayConfig {
    fn default() -> Self {
        CondDisplayConfig {
            enabled: true,
            color: ColorF {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
        }
    }
}

struct_with_serialize! {
    #[derive(Default)]
    pub struct CondsDisplayConfig {
        pub disguised: CondDisplayConfig,
        pub taunting: CondDisplayConfig,
        pub zoomed: CondDisplayConfig,
        pub invisible: CondDisplayConfig,
        pub milked: CondDisplayConfig,
        pub mg: CondDisplayConfig,
        pub butter: CondDisplayConfig,
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
        pub fov: i32,
        pub draw_fov: bool,
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
        if let Some(pos) = w.iter().position(|s| s == "default") {
            w.swap(0, pos);
        }
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
