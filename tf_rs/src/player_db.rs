use std::{
    collections::HashMap,
    env, fs,
    sync::RwLock,
};

use once_cell::sync::Lazy;

use crate::{
    config::PlayerCategoryColors,
    types::rgba::ColorF,
};

static DB: Lazy<RwLock<HashMap<String, PlayerCategory>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

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

fn get_path() -> Option<std::path::PathBuf> {
    let home = env::var("HOME").ok()?;
    let mut path = std::path::PathBuf::from(home);
    path.push(".tf_rs_players.cfg");
    Some(path)
}

pub fn load() {
    let Some(path) = get_path() else { return };
    let Ok(contents) = fs::read_to_string(&path) else { return };

    let mut db = DB.write().unwrap();
    db.clear();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
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
    let db = DB.read().unwrap();

    let contents: String = db
        .iter()
        .map(|(guid, cat)| format!("{}|{}\n", guid, cat.name()))
        .collect();

    let _ = fs::write(&path, contents);
}
