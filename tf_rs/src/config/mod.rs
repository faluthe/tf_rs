use std::{
    env, fs,
    path::PathBuf,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use log::{error, info, warn};
use once_cell::sync::Lazy;

use crate::{struct_with_serialize, types::Color};

static C: Lazy<RwLock<Config>> = Lazy::new(|| {
    let cfg = Config::load_or_create("default").unwrap_or_else(|e| {
        error!("Failed to load default config, using defaults: {e:#}");
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
        pub no_zoom: bool,
        pub remove_scope: bool,
        pub colors: ColorsConfig,
        pub menu: MenuConfig,
    }
}

// theme indexes `features::theme::THEMES`, font indexes `nuklear::Font::ALL`
struct_with_serialize! {
    #[derive(Default)]
    pub struct MenuConfig {
        pub theme: usize,
        pub font: usize,
    }
}

struct_with_serialize! {
    pub struct EspColorConfig {
        pub use_team_colors: bool,
        pub enemy: Color,
        pub friendly: Color,
    }
}

impl Default for EspColorConfig {
    fn default() -> Self {
        EspColorConfig {
            use_team_colors: true,
            // Match rgba::RED (220, 45, 35)
            enemy: Color::hex(0xdc2d23),
            // Match rgba::BLUE (40, 110, 240)
            friendly: Color::hex(0x286ef0),
        }
    }
}

struct_with_serialize! {
    pub struct BuildingColorConfig {
        pub enemy: Color,
        pub friendly: Color,
    }
}

impl Default for BuildingColorConfig {
    fn default() -> Self {
        BuildingColorConfig {
            enemy: Color::hex(0xdc2d23),
            friendly: Color::hex(0x286ef0),
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
    pub struct PlayerCategoryColors {
        pub category1: Color,
        pub category2: Color,
        pub category3: Color,
        pub category4: Color,
    }
}

impl Default for PlayerCategoryColors {
    fn default() -> Self {
        PlayerCategoryColors {
            // purple
            category1: Color::hex(0xa020f0),
            // yellow
            category2: Color::hex(0xffeb04),
            // cyan
            category3: Color::hex(0x00eded),
            // pink
            category4: Color::hex(0xff69b4),
        }
    }
}

struct_with_serialize! {
    pub struct ColorsConfig {
        pub boxes: EspColorConfig,
        pub names: EspColorConfig,
        pub buildings: BuildingsColorsConfig,
        pub player_categories: PlayerCategoryColors,
    }
}

impl Default for ColorsConfig {
    fn default() -> Self {
        ColorsConfig {
            boxes: EspColorConfig::default(),
            names: EspColorConfig::default(),
            buildings: BuildingsColorsConfig::default(),
            player_categories: PlayerCategoryColors::default(),
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
        pub ignore_invis: bool,
        pub chams: ChamsConfig,
    }
}

struct_with_serialize! {
    pub struct ChamsConfig {
        pub enabled: bool,
        pub material: usize,
        pub visible: Color,
        pub occluded: Color,
    }
}

impl Default for ChamsConfig {
    fn default() -> Self {
        ChamsConfig {
            enabled: false,
            material: 0,
            visible: Color::hex(0x00ff00),
            occluded: Color::hex(0xff0000),
        }
    }
}

struct_with_serialize! {
    pub struct CondDisplayConfig {
        pub enabled: bool,
        pub color: Color,
    }
}

impl Default for CondDisplayConfig {
    fn default() -> Self {
        CondDisplayConfig {
            enabled: true,
            color: Color::hex(0xffffff),
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
        pub category: bool,
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
        pub ignore_category: u8,
        pub priority_category: u8,
        pub disable_when_spectated: usize,
        pub ignore_invis: bool,
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
        info!("Refreshed config list, found {}", w.len());
    }

    // TODO: Clean this up
    fn get_config_names() -> Vec<String> {
        let home = match env::var("HOME") {
            Ok(h) => h,
            Err(e) => {
                warn!("Can't list configs, HOME is unset: {e}");
                return Vec::new();
            }
        };
        let mut configs = Vec::new();
        let dir = match fs::read_dir(&home) {
            Ok(d) => d,
            Err(e) => {
                warn!("Can't list configs in {home}: {e}");
                return Vec::new();
            }
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

        let s = fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {e}", path.display()))?;
        *self = s
            .parse::<Config>()
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {e}", path.display()))?;

        info!("Loaded config {name}");
        Ok(())
    }

    pub fn save(&self, name: &str) -> anyhow::Result<()> {
        let path = Config::get_path(name)?;

        fs::write(&path, self.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {e}", path.display()))?;

        info!("Saved config {name}");
        Ok(())
    }

    fn load_or_create(name: &str) -> anyhow::Result<Self> {
        let path = Config::get_path(name)?;

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(s) => match s.parse::<Config>() {
                    Ok(cfg) => {
                        info!("Loaded config {name}");
                        return Ok(cfg);
                    }
                    Err(e) => warn!(
                        "Failed to parse {}, overwriting with defaults: {e}",
                        path.display()
                    ),
                },
                Err(e) => warn!(
                    "Failed to read {}, overwriting with defaults: {e}",
                    path.display()
                ),
            }
        }

        let cfg = Config::default();

        fs::write(&path, cfg.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {e}", path.display()))?;
        info!("Created config {name} at {}", path.display());

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
