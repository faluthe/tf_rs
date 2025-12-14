use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{
    interfaces::Interfaces,
    types::{BBox, Entity, Player, Vec2, Vec3},
};

pub mod macros;

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
        .map(|v| Interfaces::debug_overlay().screen_position(&v))
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
    let trace = Interfaces::engine_trace().trace_ray(from, to, 0x4200400b, Some(ignore_entity));
    trace.fraction >= 0.97 || trace.entity == ent.this || trace.end_pos == *to
}

pub fn is_pos_visible(from: &Vec3, to: &Vec3, ignore_entity: &Player) -> bool {
    let trace = Interfaces::engine_trace().trace_ray(from, to, 0x4200400b, Some(ignore_entity));
    trace.fraction >= 0.97 || trace.end_pos == *to
}

pub fn pattern_scan(lib: &str, pattern: &str) -> Option<usize> {
    let file = File::open(format!("/proc/self/maps")).ok()?;
    let reader = BufReader::new(file);

    let pattern: Vec<Option<u8>> = pattern
        .split_whitespace()
        .map(|b| {
            if b == "?" || b == "??" {
                None
            } else {
                Some(u8::from_str_radix(b, 16).ok()?)
            }
        })
        .collect();

    for line in reader.lines() {
        let l = line.ok()?;

        if !l.contains(lib) {
            continue;
        }

        let mut line = l.split_whitespace();

        let range = line.next()?;
        let perms = line.next()?;

        if !perms.starts_with('r') {
            continue;
        }

        let (start, end) = range.split_once('-')?;
        let start = usize::from_str_radix(start, 16).ok()? as *const u8;
        let end = usize::from_str_radix(end, 16).ok()? as *const u8;

        let mut cur = start;

        while unsafe { cur.add(pattern.len()) } < end {
            let mut found = true;

            for (i, byte) in pattern.iter().enumerate() {
                if let Some(b) = byte {
                    let memory_byte = unsafe { *cur.add(i) };
                    if memory_byte != *b {
                        found = false;
                        break;
                    }
                }
            }

            if found {
                return Some(cur as usize);
            }

            cur = unsafe { cur.add(1) };
        }
    }

    None
}

pub fn projectile_fire_setup(view_angles: &Vec3, shoot_pos: &Vec3, offset: &Vec3) -> Vec3 {
    let (forward, right, up) = angles_to_direction_vector(view_angles);

    Vec3 {
        x: shoot_pos.x + (forward.x * offset.x) + (right.x * offset.y) + (up.x * offset.z),
        y: shoot_pos.y + (forward.y * offset.x) + (right.y * offset.y) + (up.y * offset.z),
        z: shoot_pos.z + (forward.z * offset.x) + (right.z * offset.y) + (up.z * offset.z),
    }
}

pub fn angles_to_direction_vector(angles: &Vec3) -> (Vec3, Vec3, Vec3) {
    let (sp, cp) = angles.x.to_radians().sin_cos(); // pitch
    let (sy, cy) = angles.y.to_radians().sin_cos(); // yaw
    let (sr, cr) = angles.z.to_radians().sin_cos(); // roll

    let forward = Vec3 {
        x: cp * cy,
        y: cp * sy,
        z: -sp,
    };

    let right = Vec3 {
        x: (-sr * sp * cy + -cr * -sy),
        y: (-sr * sp * sy + -cr * cy),
        z: -sr * cp,
    };

    let up = Vec3 {
        x: (cr * sp * cy + -sr * -sy),
        y: (cr * sp * sy + -sr * cy),
        z: cr * cp,
    };

    (forward, right, up)
}
