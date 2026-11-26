use std::{
    ffi::{CStr, CString, OsStr, c_void},
    io::Write,
    mem,
    os::unix::ffi::OsStrExt,
    path::Path,
};

use crate::{
    interfaces::{Interfaces, mask},
    types::{BBox, Entity, Flow, Player, Vec2, Vec3},
};

pub mod macros;
pub mod module;
pub mod view;

/// Index and player of each enemy of `localplayer` that's alive and not dormant, leaving out
/// invisible ones with `ignore_invis`
pub fn enemies(localplayer: &Player, ignore_invis: bool) -> impl Iterator<Item = (i32, Player)> {
    let team = localplayer.team();
    let local = localplayer.index();
    let list = Interfaces::entity_list();
    (1..=Interfaces::engine_client().get_max_clients())
        .filter(move |&i| i != local)
        .filter_map(move |i| Some((i, list.get_client_entity::<Player>(i)?)))
        .filter(move |(_, p)| {
            !p.is_dormant()
                && !p.is_dead()
                && p.team() != team
                && !(ignore_invis && p.is_invisible())
        })
}

/// Total real latency, how long until the server runs our command plus how old what we see is
pub fn round_trip() -> f32 {
    Interfaces::engine_client()
        .get_net_channel_info()
        .map_or(0.0, |nci| {
            nci.latency(Flow::Outgoing) + nci.latency(Flow::Incoming)
        })
}

/// Append `rows` to the CSV file `name` next to this library, starting it with `header` when
/// it's new
pub fn append_csv(name: &str, header: &str, rows: &str) {
    let Some(lib) = lib_path() else {
        return;
    };
    let Some(dir) = Path::new(OsStr::from_bytes(lib.as_bytes())).parent() else {
        return;
    };
    let path = dir.join(name);
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut f) => {
            let empty = f.metadata().is_ok_and(|m| m.len() == 0);
            let header = if empty { header } else { "" };
            let _ = f.write_all(format!("{header}{rows}").as_bytes());
        }
        Err(e) => log::warn!("Could not write {}: {e}", path.display()),
    }
}

/// Path this library was loaded from, as passed to dlopen.
pub fn lib_path() -> Option<CString> {
    unsafe {
        let mut info: libc::Dl_info = mem::zeroed();
        if libc::dladdr(lib_path as *const c_void, &mut info) == 0 || info.dli_fname.is_null() {
            return None;
        }
        Some(CStr::from_ptr(info.dli_fname).to_owned())
    }
}

pub fn get_localplayer() -> Option<Player> {
    let index = Interfaces::engine_client().get_localplayer_index();

    if index < 1 {
        return None;
    }

    Interfaces::entity_list().get_client_entity(index)
}

pub fn get_bounding_box(entity: &Entity) -> Option<BBox> {
    let shrink_xy = 0.75;
    let shrink_z = 0.95;
    let origin = entity.origin();
    let mins = entity.mins();
    let maxs = entity.maxs();

    let mins = Vec3 {
        x: mins.x * shrink_xy,
        y: mins.y * shrink_xy,
        z: mins.z * shrink_z,
    };
    let maxs = Vec3 {
        x: maxs.x * shrink_xy,
        y: maxs.y * shrink_xy,
        z: maxs.z * shrink_z,
    };

    let offsets = [
        (maxs.x, maxs.y, maxs.z), // frt
        (mins.x, mins.y, mins.z), // blb
        (maxs.x, mins.y, maxs.z), // brt
        (mins.x, maxs.y, mins.z), // flb
        (maxs.x, mins.y, mins.z), // brb
        (mins.x, maxs.y, maxs.z), // flt
        (mins.x, mins.y, maxs.z), // blt
        (maxs.x, maxs.y, mins.z), // frb
    ];

    let points_2d: Vec<Vec2> = offsets
        .into_iter()
        .map(|(ox, oy, oz)| Vec3 {
            x: origin.x + ox,
            y: origin.y + oy,
            z: origin.z + oz,
        })
        .map(|v| view::world_to_screen(&v))
        .collect::<Option<Vec<_>>>()?; // None if any projection failed

    let first = points_2d[0];
    let (mut left, mut right) = (first.x, first.x);
    let (mut top, mut bottom) = (first.y, first.y);

    for p in &points_2d[1..] {
        left = left.min(p.x);
        right = right.max(p.x);
        top = top.min(p.y);
        bottom = bottom.max(p.y);
    }

    Some(BBox {
        left: left as i32,
        top: top as i32,
        right: right as i32,
        bottom: bottom as i32,
    })
}

pub fn calculate_angle(from: &Vec3, to: &Vec3) -> Vec3 {
    let delta = Vec3 {
        x: to.x - from.x,
        y: to.y - from.y,
        z: to.z - from.z,
    };

    // Common side between the two right triangles
    let hyp = ((delta.x * delta.x) + (delta.y * delta.y)).sqrt();
    let pitch = delta.z.atan2(hyp).to_degrees();
    let yaw = delta.y.atan2(delta.x).to_degrees();
    Vec3 {
        x: -pitch,
        y: yaw,
        z: 0.0,
    }
}

pub fn is_ent_visible(from: &Vec3, to: &Vec3, ent: &Entity, ignore_entity: &Player) -> bool {
    let trace = Interfaces::engine_trace().trace_ray(from, to, mask::BULLET, Some(ignore_entity));
    trace.fraction >= 0.97 || trace.entity == ent.this || trace.end_pos == *to
}

/// How far along `start` to `end` it enters the box `mins`..`maxs` grown by `grow`, as a fraction
/// of the segment, zero when it starts inside
pub fn segment_box_entry(
    start: &Vec3,
    end: &Vec3,
    mins: &Vec3,
    maxs: &Vec3,
    grow: f32,
) -> Option<f32> {
    let axes = [
        (start.x, end.x, mins.x, maxs.x),
        (start.y, end.y, mins.y, maxs.y),
        (start.z, end.z, mins.z, maxs.z),
    ];
    let (mut enter, mut exit) = (0.0f32, 1.0f32);
    for (s, e, lo, hi) in axes {
        let (lo, hi) = (lo - grow, hi + grow);
        let d = e - s;
        if d.abs() < f32::EPSILON {
            if s < lo || s > hi {
                return None;
            }
            continue;
        }
        let (t0, t1) = ((lo - s) / d, (hi - s) / d);
        enter = enter.max(t0.min(t1));
        exit = exit.min(t0.max(t1));
        if enter > exit {
            return None;
        }
    }
    Some(enter)
}
