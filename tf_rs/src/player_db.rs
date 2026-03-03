use std::{collections::HashMap, env, fs, sync::RwLock};

use once_cell::sync::Lazy;

use crate::{config::PlayerCategoryColors, types::rgba::ColorF};

static DB: Lazy<RwLock<HashMap<String, PlayerCategory>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

static NAMES: Lazy<RwLock<[String; 4]>> = Lazy::new(|| {
    RwLock::new([
        "Cat 1".to_string(),
        "Cat 2".to_string(),
        "Cat 3".to_string(),
        "Cat 4".to_string(),
    ])
});

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayerCategory {
    None,
    Category1,
    Category2,
    Category3,
    Category4,
}

impl PlayerCategory {
    pub fn name(self) -> &'static str {
        match self {
            PlayerCategory::None => "None",
            PlayerCategory::Category1 => "Cat 1",
            PlayerCategory::Category2 => "Cat 2",
            PlayerCategory::Category3 => "Cat 3",
            PlayerCategory::Category4 => "Cat 4",
        }
    }

    pub fn display_name(self) -> String {
        let names = NAMES.read().unwrap();
        match self {
            PlayerCategory::None => "None".to_string(),
            PlayerCategory::Category1 => names[0].clone(),
            PlayerCategory::Category2 => names[1].clone(),
            PlayerCategory::Category3 => names[2].clone(),
            PlayerCategory::Category4 => names[3].clone(),
        }
    }

    pub fn next(self) -> Self {
        match self {
            PlayerCategory::None => PlayerCategory::Category1,
            PlayerCategory::Category1 => PlayerCategory::Category2,
            PlayerCategory::Category2 => PlayerCategory::Category3,
            PlayerCategory::Category3 => PlayerCategory::Category4,
            PlayerCategory::Category4 => PlayerCategory::None,
        }
    }

    pub fn color(self, colors: &PlayerCategoryColors) -> ColorF {
        match self {
            PlayerCategory::None => ColorF::default(),
            PlayerCategory::Category1 => colors.category1,
            PlayerCategory::Category2 => colors.category2,
            PlayerCategory::Category3 => colors.category3,
            PlayerCategory::Category4 => colors.category4,
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "Cat 1" => PlayerCategory::Category1,
            "Cat 2" => PlayerCategory::Category2,
            "Cat 3" => PlayerCategory::Category3,
            "Cat 4" => PlayerCategory::Category4,
            _ => PlayerCategory::None,
        }
    }
}

pub fn get(guid: &str) -> PlayerCategory {
    DB.read()
        .unwrap()
        .get(guid)
        .copied()
        .unwrap_or(PlayerCategory::None)
}

pub fn set(guid: &str, category: PlayerCategory) {
    {
        let mut db = DB.write().unwrap();
        if category == PlayerCategory::None {
            db.remove(guid);
        } else {
            db.insert(guid.to_string(), category);
        }
    }
    save();
}

pub fn get_names() -> [String; 4] {
    NAMES.read().unwrap().clone()
}

pub fn set_name(index: usize, name: &str) {
    if index < 4 {
        NAMES.write().unwrap()[index] = name.to_string();
        save();
    }
}

fn get_path() -> Option<std::path::PathBuf> {
    let home = env::var("HOME").ok()?;
    let mut path = std::path::PathBuf::from(home);
    path.push(".tf_rs_players.cfg");
    Some(path)
}

pub fn load() {
    let Some(path) = get_path() else { return };
    let Ok(contents) = fs::read_to_string(&path) else {
        return;
    };

    // Parse header lines (category names) — release lock before touching DB
    {
        let mut names = NAMES.write().unwrap();
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(header) = line.strip_prefix('#') {
                if let Some((key, val)) = header.split_once('=') {
                    let idx = match key {
                        "cat1" => Some(0),
                        "cat2" => Some(1),
                        "cat3" => Some(2),
                        "cat4" => Some(3),
                        _ => None,
                    };
                    if let Some(i) = idx {
                        names[i] = val.to_string();
                    }
                }
            }
        }
    }

    // Parse GUID entries
    let mut db = DB.write().unwrap();
    db.clear();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((guid, cat_name)) = line.split_once('|') {
            let cat = PlayerCategory::from_str(cat_name);
            if cat != PlayerCategory::None {
                db.insert(guid.to_string(), cat);
            }
        }
    }
}

fn save() {
    let Some(path) = get_path() else { return };

    let mut contents = String::new();

    {
        let names = NAMES.read().unwrap();
        for (i, name) in names.iter().enumerate() {
            contents.push_str(&format!("#cat{}={}\n", i + 1, name));
        }
    }

    {
        let db = DB.read().unwrap();
        for (guid, cat) in db.iter() {
            contents.push_str(&format!("{}|{}\n", guid, cat.name()));
        }
    }

    let _ = fs::write(&path, contents);
}
