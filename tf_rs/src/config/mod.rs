//! Feature settings, saved to `~/.<name>.tf_rs.cfg`. Nested structs are indented blocks:
//!
//! ```text
//! esp:
//!     master: true
//!     player_enemy:
//!         boxes: true
//! ```

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
    #[derive(Default, Clone)]
    pub struct Config {
        pub bunnyhop: bool,
        pub esp: ESPConfig,
        pub aimbot: AimbotConfig,
        pub backtrack: BacktrackConfig,
        pub thirdperson: KeyConfig,
        pub spectator_list: bool,
        pub no_zoom: bool,
        pub remove_scope: bool,
        pub colors: ColorsConfig,
        pub menu: MenuConfig,
    }
}

struct_with_serialize! {
    #[derive(Default, Clone)]
    pub struct MenuConfig {
        /// Index into `features::theme::THEMES`
        pub theme: usize,
        /// Index into `nuklear::Font::ALL`
        pub font: usize,
    }
}

struct_with_serialize! {
    #[derive(Clone)]
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
    #[derive(Clone)]
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
    #[derive(Clone)]
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
    #[derive(Clone)]
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
    #[derive(Default, Clone)]
    pub struct ColorsConfig {
        pub boxes: EspColorConfig,
        pub names: EspColorConfig,
        pub buildings: BuildingsColorsConfig,
        pub player_categories: PlayerCategoryColors,
    }
}

struct_with_serialize! {
    #[derive(Default, Clone)]
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
    #[derive(Clone)]
    pub struct ChamsConfig {
        pub enabled: bool,
        pub enemy: ChamsGroupConfig,
        pub friendly: ChamsGroupConfig,
        pub local: ChamsGroupConfig,
        pub building_enemy: ChamsGroupConfig,
        pub building_friendly: ChamsGroupConfig,
        pub category1: ChamsGroupConfig,
        pub category2: ChamsGroupConfig,
        pub category3: ChamsGroupConfig,
        pub category4: ChamsGroupConfig,
        pub hands: ChamsGroupConfig,
        pub weapon: ChamsGroupConfig,
        pub backtrack: ChamsGroupConfig,
        pub backtrack_target: ChamsGroupConfig,
        pub aimbot_target: ChamsTargetConfig,
    }
}

impl Default for ChamsConfig {
    fn default() -> Self {
        let categories = PlayerCategoryColors::default();
        ChamsConfig {
            enabled: false,
            enemy: ChamsGroupConfig::new(true, 0xff4040, 0x9a0000),
            friendly: ChamsGroupConfig::new(false, 0x40ff40, 0x008a00),
            local: ChamsGroupConfig::new(false, 0xffffff, 0x808080),
            building_enemy: ChamsGroupConfig::new(false, 0xffa500, 0x9a5a00),
            building_friendly: ChamsGroupConfig::new(false, 0x40c0ff, 0x005f8a),
            category1: ChamsGroupConfig::from_color(categories.category1),
            category2: ChamsGroupConfig::from_color(categories.category2),
            category3: ChamsGroupConfig::from_color(categories.category3),
            category4: ChamsGroupConfig::from_color(categories.category4),
            hands: ChamsGroupConfig::new(false, 0x8080ff, 0x8080ff),
            weapon: ChamsGroupConfig::new(false, 0xffffff, 0xffffff),
            backtrack: ChamsGroupConfig {
                visible_color: Color {
                    a: 90,
                    ..Color::hex(0xffffff)
                },
                occluded_color: Color {
                    a: 50,
                    ..Color::hex(0xffffff)
                },
                ..ChamsGroupConfig::new(false, 0, 0)
            },
            backtrack_target: ChamsGroupConfig {
                visible_color: Color {
                    a: 140,
                    ..Color::hex(0xffd000)
                },
                occluded_color: Color {
                    a: 90,
                    ..Color::hex(0xffd000)
                },
                ..ChamsGroupConfig::new(false, 0, 0)
            },
            aimbot_target: ChamsTargetConfig::default(),
        }
    }
}

struct_with_serialize! {
    // Colors the aimbot target's chams are drawn in instead of its group's, with its group's
    // materials
    #[derive(Clone, Copy)]
    pub struct ChamsTargetConfig {
        pub visible: bool,
        pub occluded: bool,
        pub visible_color: Color,
        pub occluded_color: Color,
    }
}

impl Default for ChamsTargetConfig {
    fn default() -> Self {
        ChamsTargetConfig {
            visible: false,
            occluded: false,
            visible_color: Color::hex(0xffd000),
            occluded_color: Color::hex(0x9a7d00),
        }
    }
}

struct_with_serialize! {
    #[derive(Clone, Copy)]
    pub struct ChamsLayerConfig {
        // Index into chams::MATERIALS
        pub material: usize,
        // Draw where the entity can be seen
        pub visible: bool,
        // Draw through walls
        pub occluded: bool,
        pub visible_color: Color,
        pub occluded_color: Color,
    }
}

impl Default for ChamsLayerConfig {
    fn default() -> Self {
        ChamsLayerConfig {
            material: 0,
            visible: false,
            occluded: false,
            visible_color: Color::hex(0xffffff),
            occluded_color: Color::hex(0xffffff),
        }
    }
}

/// Mutable access to one layer of a group
pub struct ChamsLayerMut<'a> {
    pub material: &'a mut usize,
    pub visible: &'a mut bool,
    pub occluded: &'a mut bool,
    pub visible_color: &'a mut Color,
    pub occluded_color: &'a mut Color,
}

impl ChamsLayerConfig {
    fn as_mut(&mut self) -> ChamsLayerMut<'_> {
        ChamsLayerMut {
            material: &mut self.material,
            visible: &mut self.visible,
            occluded: &mut self.occluded,
            visible_color: &mut self.visible_color,
            occluded_color: &mut self.occluded_color,
        }
    }
}

struct_with_serialize! {
    #[derive(Clone, Copy)]
    pub struct ChamsGroupConfig {
        /// The first layer, its fields stay at this level so configs from before layers load.
        /// Without a visible pass on any layer the normal model is drawn where it can be seen
        pub material: usize,
        pub visible: bool,
        pub occluded: bool,
        pub visible_color: Color,
        pub occluded_color: Color,
        // Drawn on top of the first layer, in order
        pub overlay1: ChamsLayerConfig,
        pub overlay2: ChamsLayerConfig,
    }
}

impl ChamsGroupConfig {
    pub const LAYERS: usize = 3;

    fn new(enabled: bool, visible: u32, occluded: u32) -> Self {
        ChamsGroupConfig {
            material: 0,
            visible: enabled,
            occluded: enabled,
            visible_color: Color::hex(visible),
            occluded_color: Color::hex(occluded),
            overlay1: ChamsLayerConfig::default(),
            overlay2: ChamsLayerConfig::default(),
        }
    }

    fn from_color(color: Color) -> Self {
        let dim = |c: u8| (c as u16 * 3 / 5) as u8;
        ChamsGroupConfig {
            visible_color: color,
            occluded_color: Color {
                r: dim(color.r),
                g: dim(color.g),
                b: dim(color.b),
                a: color.a,
            },
            ..ChamsGroupConfig::new(false, 0, 0)
        }
    }

    /// Every layer, bottom first
    pub fn layers(&self) -> [ChamsLayerConfig; Self::LAYERS] {
        [
            ChamsLayerConfig {
                material: self.material,
                visible: self.visible,
                occluded: self.occluded,
                visible_color: self.visible_color,
                occluded_color: self.occluded_color,
            },
            self.overlay1,
            self.overlay2,
        ]
    }

    pub fn layer_mut(&mut self, index: usize) -> ChamsLayerMut<'_> {
        match index {
            0 => ChamsLayerMut {
                material: &mut self.material,
                visible: &mut self.visible,
                occluded: &mut self.occluded,
                visible_color: &mut self.visible_color,
                occluded_color: &mut self.occluded_color,
            },
            1 => self.overlay1.as_mut(),
            _ => self.overlay2.as_mut(),
        }
    }

    pub fn active(&self) -> bool {
        self.layers().iter().any(|l| l.visible || l.occluded)
    }
}

impl Default for ChamsGroupConfig {
    fn default() -> Self {
        ChamsGroupConfig::new(false, 0xffffff, 0xffffff)
    }
}

struct_with_serialize! {
    #[derive(Clone)]
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
    #[derive(Default, Clone)]
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
    #[derive(Default, Clone)]
    pub struct EntityESPConfig {
        pub boxes: bool,
        pub names: bool,
        pub health: bool,
        pub conds: bool,
    }
}

struct_with_serialize! {
    #[derive(Clone)]
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
        pub melee: bool,
        pub auto_swing: bool,
        pub swing_prediction: bool,
        pub auto_backstab: bool,
        /// Predict players turning as they move on the ground
        pub strafe_ground: bool,
        /// Predict players turning as they move in the air
        pub strafe_air: bool,
        pub projectile: bool,
        /// How far ahead to simulate targets for projectiles, in milliseconds
        pub max_sim_time: i32,
        /// Hit surfaces near targets that explosives can't hit directly
        pub splash: bool,
        /// Hold fire on a target until our explosive already on its way to them lands
        pub wait_for_shot: bool,
        pub auto_detonate: bool,
        pub draw_prediction: bool,
        /// Also draw where a shot along the view would go, and the shot the aimbot would take
        pub draw_prediction_always: bool,
        /// Write how far movement prediction is off to prediction.csv
        pub log_prediction: bool,
    }
}

struct_with_serialize! {
    #[derive(Clone)]
    pub struct BacktrackConfig {
        pub enabled: bool,
        pub length: i32,
        pub fake_ping: i32,
        pub disable_interpolation: bool,
    }
}

impl Default for AimbotConfig {
    fn default() -> Self {
        AimbotConfig {
            master: false,
            silent_aim: false,
            building_aim: false,
            key: KeyConfig::default(),
            fov: 0,
            draw_fov: false,
            ignore_category: 0,
            priority_category: 0,
            disable_when_spectated: 0,
            ignore_invis: false,
            melee: false,
            auto_swing: false,
            swing_prediction: false,
            auto_backstab: false,
            strafe_ground: true,
            strafe_air: true,
            projectile: false,
            max_sim_time: 2000,
            splash: true,
            wait_for_shot: false,
            auto_detonate: false,
            draw_prediction: false,
            draw_prediction_always: false,
            log_prediction: false,
        }
    }
}

impl Default for BacktrackConfig {
    fn default() -> Self {
        BacktrackConfig {
            enabled: false,
            length: 200,
            fake_ping: 0,
            disable_interpolation: true,
        }
    }
}

struct_with_serialize! {
    #[derive(Default, Clone)]
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
        for entry in dir.flatten() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with('.') && file_name.ends_with(".tf_rs.cfg") {
                configs.push(file_name.to_string());
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
