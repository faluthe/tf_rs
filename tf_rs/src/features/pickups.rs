//! Health packs and ammo boxes. The server stops sending one while it's picked up and sends it
//! again when it comes back ITEM_RESPAWN_TIME later, so on the client it goes dormant. One
//! going dormant with a player touching it was picked up, and is back that long after

use std::{collections::HashMap, sync::Mutex};

use crate::{
    config::{Config, PickupESPConfig},
    helpers,
    interfaces::{Interfaces, Surface},
    types::{ClassId, Entity, Player, Vec3},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Health,
    Ammo,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::Health => "Health",
            Kind::Ammo => "Ammo",
        }
    }
}

/// Seconds a pickup is gone for, tf_gamerules.cpp
const RESPAWN_TIME: f32 = 10.0;

/// What kind of pickup `entity` is, if it's one that respawns where it is
pub fn kind(entity: &Entity) -> Option<Kind> {
    if !matches!(entity.class_id(), Some(ClassId::AmmoHealth)) {
        return None;
    }
    let model = entity.renderable().model();
    if model.is_null() {
        return None;
    }
    let model_info = Interfaces::model_info();
    let name = model_info.get_model_name(model);
    if name.contains("medkit") {
        Some(Kind::Health)
    } else if name.contains("ammopack") {
        Some(Kind::Ammo)
    } else {
        None
    }
}

struct Tracked {
    dormant: bool,
    /// Realtime it comes back at, while it's picked up
    respawn: Option<f32>,
}

static TRACKED: Mutex<Option<HashMap<i32, Tracked>>> = Mutex::new(None);

/// Whether a player standing at `player` is touching a pickup at `item`
fn touching(item: &Vec3, player: &Vec3) -> bool {
    let d = *player - *item;
    d.x.abs() < 56.0 && d.y.abs() < 56.0 && d.z > -96.0 && d.z < 64.0
}

/// Note the pickups taken since the last update, call once the new entity data is in
pub fn update() {
    let list = Interfaces::entity_list();
    let now = Interfaces::global_vars().realtime;
    let players: Vec<Vec3> = (1..=Interfaces::engine_client().get_max_clients())
        .filter_map(|i| list.get_client_entity::<Player>(i))
        .filter(|p| !p.is_dormant() && !p.is_dead())
        .map(|p| p.ent.origin())
        .collect();

    let mut tracked = TRACKED.lock().unwrap_or_else(|e| e.into_inner());
    let tracked = tracked.get_or_insert_with(HashMap::new);
    let mut seen = Vec::new();
    for i in 1..=list.max_entities() {
        let Some(entity) = list.get_client_entity::<Entity>(i) else {
            continue;
        };
        if kind(&entity).is_none() {
            continue;
        }
        seen.push(i);
        let dormant = entity.is_dormant();
        let t = tracked.entry(i).or_insert(Tracked {
            dormant,
            respawn: None,
        });
        if dormant && !t.dormant {
            let origin = entity.origin();
            if players.iter().any(|p| touching(&origin, p)) {
                t.respawn = Some(now + RESPAWN_TIME);
            }
        } else if !dormant {
            t.respawn = None;
        }
        t.dormant = dormant;
    }
    tracked.retain(|i, t| {
        if t.respawn.is_some_and(|r| r <= now) {
            t.respawn = None;
        }
        seen.contains(i)
    });
}

/// Seconds until pickup `index` is back, while it's picked up
fn respawn_in(index: i32, now: f32) -> Option<f32> {
    let tracked = TRACKED.lock().unwrap_or_else(|e| e.into_inner());
    let respawn = tracked.as_ref()?.get(&index)?.respawn?;
    (respawn > now).then_some(respawn - now)
}

/// Label each pickup, and how long until it's back while it's picked up
pub fn draw(surface: &Surface, config: &Config) {
    if !config.esp.master {
        return;
    }
    let esp = &config.esp;
    if !esp.health.bool() && !esp.ammo.bool() {
        return;
    }
    let list = Interfaces::entity_list();
    let now = Interfaces::global_vars().realtime;
    for i in 1..=list.max_entities() {
        let Some(entity) = list.get_client_entity::<Entity>(i) else {
            continue;
        };
        let Some(kind) = kind(&entity) else {
            continue;
        };
        let cfg: &PickupESPConfig = match kind {
            Kind::Health => &esp.health,
            Kind::Ammo => &esp.ammo,
        };
        let text = if entity.is_dormant() {
            let Some(left) = respawn_in(i, now).filter(|_| cfg.timer) else {
                continue;
            };
            if cfg.names {
                format!("{} {left:.1}s", kind.label())
            } else {
                format!("{left:.1}s")
            }
        } else if cfg.names {
            kind.label().to_string()
        } else {
            continue;
        };

        let top = entity.origin() + Vec3::new(0.0, 0.0, entity.maxs().z + 8.0);
        let Some(at) = helpers::view::world_to_screen(&top) else {
            continue;
        };
        // About half the width of the text in the ESP font
        let x = at.x - text.len() as f32 * 4.0;
        if x < 0.0 || at.y < 0.0 {
            continue;
        }
        surface.draw_set_text_color_c(&cfg.color);
        surface.draw_set_text_pos(x as u32, at.y as u32);
        surface.draw_print_text(&text);
    }
}
