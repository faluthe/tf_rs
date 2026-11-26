//! Our projectiles in the air and the shots that fired them

use std::sync::Mutex;

use crate::{
    helpers,
    interfaces::Interfaces,
    types::{ClassId, Entity, Player},
};

/// Seconds a shot waits for its projectile to show up before it's taken as never fired
const SPAWN_WINDOW: f32 = 0.5;

/// An explosive we fired that's still on its way to a target
struct InFlight {
    target: i32,
    /// Server time it lands
    land_time: f32,
    /// Realtime it was fired at
    fired: f32,
    /// Its entity handle once it's shown up
    projectile: Option<u32>,
    /// Realtime after which it's taken as a miss
    expires: f32,
}

#[derive(Default)]
struct Tracking {
    in_flight: Vec<InFlight>,
    /// Our projectiles last time, so new ones can be told apart
    known: Vec<u32>,
}

/// Our explosives in the air. A blast throws its target, so a shot that lands after one of these
/// is aimed at where the target would have been without it.
static TRACKING: Mutex<Tracking> = Mutex::new(Tracking {
    in_flight: Vec::new(),
    known: Vec::new(),
});

struct Scanned {
    handle: u32,
    index: i32,
}

/// Our projectiles and the local player they were scanned for. They only change with network
/// updates, so the scan is reused until `forget_projectiles`.
static SCANNED: Mutex<Option<(i32, Vec<Scanned>)>> = Mutex::new(None);

/// Projectiles may have come or gone, the next caller scans again
pub fn forget_projectiles() {
    *SCANNED.lock().unwrap() = None;
}

/// Our projectiles that are out, by entity handle
pub fn our_projectiles(local: i32) -> Vec<(u32, Entity)> {
    let list = Interfaces::entity_list();
    let mut scanned = SCANNED.lock().unwrap();
    if scanned.as_ref().is_none_or(|(l, _)| *l != local) {
        *scanned = Some((local, scan_projectiles(local)));
    }
    let (_, found) = scanned.as_ref().unwrap();
    // Looked up again by index, a handle that no longer matches is a different entity now
    found
        .iter()
        .filter_map(|s| {
            let ent = list.get_client_entity::<Entity>(s.index)?;
            (ent.ref_handle() == s.handle).then_some((s.handle, ent))
        })
        .collect()
}

fn scan_projectiles(local: i32) -> Vec<Scanned> {
    let list = Interfaces::entity_list();
    let mut out = Vec::new();
    for i in 1..=list.max_entities() {
        let Some(ent) = list.get_client_entity::<Entity>(i) else {
            continue;
        };
        let class = ent.class_id();
        let projectile = matches!(
            class,
            Some(
                ClassId::Rocket
                    | ClassId::PillOrSticky
                    | ClassId::Arrow
                    | ClassId::Flare
                    | ClassId::CrossbowBolt
            )
        );
        if !projectile || ent.is_dormant() {
            continue;
        }
        let thrower = matches!(class, Some(ClassId::PillOrSticky)) && ent.thrower_index() == local;
        if ent.owner_index() == local || thrower {
            out.push(Scanned {
                handle: ent.ref_handle(),
                index: i,
            });
        }
    }
    out
}

/// Match new projectiles to the shots that made them and forget shots whose projectile is gone
pub(super) fn track_in_flight(localplayer: &Player) {
    let now = Interfaces::global_vars().realtime;
    let ours: Vec<u32> = our_projectiles(localplayer.index())
        .into_iter()
        .map(|(h, _)| h)
        .collect();
    let mut tracking = TRACKING.lock().unwrap();
    let Tracking { in_flight, known } = &mut *tracking;
    for handle in ours.iter().filter(|h| !known.contains(h)) {
        if let Some(shot) = in_flight.iter_mut().find(|f| f.projectile.is_none()) {
            shot.projectile = Some(*handle);
        }
    }
    in_flight.retain(|f| {
        now < f.expires
            && match f.projectile {
                Some(h) => ours.contains(&h),
                None => now - f.fired < SPAWN_WINDOW,
            }
    });
    *known = ours;
}

/// A shot at `target` landing at server time `land_time`, `flight` seconds from now
pub(super) fn record(target: i32, land_time: f32, flight: f32) {
    let now = Interfaces::global_vars().realtime;
    TRACKING.lock().unwrap().in_flight.push(InFlight {
        target,
        land_time,
        fired: now,
        projectile: None,
        expires: now + flight + helpers::round_trip() + 0.1,
    });
}

/// Target and landing time of each of our explosives seen flying. Unconfirmed shots don't count,
/// several commands in a frame can each look like they fire.
pub(super) fn landings() -> Vec<(i32, f32)> {
    TRACKING
        .lock()
        .unwrap()
        .in_flight
        .iter()
        .filter(|f| f.projectile.is_some())
        .map(|f| (f.target, f.land_time))
        .collect()
}
