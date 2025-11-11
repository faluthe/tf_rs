use crate::{
    interfaces::Interfaces,
    types::{Player, Vec2, Vec3},
};

pub mod macros;

pub fn get_localplayer() -> Option<Player> {
    let index = Interfaces::engine_client().get_localplayer_index();

    if index < 1 {
        return None;
    }

    Interfaces::entity_list().get_client_entity(index)
}

// left, top, right, bottom
pub fn get_bounding_box(player: Player) -> Option<(i32, i32, i32, i32)> {
    let origin = player.origin();
    let mins = player.mins();
    let maxs = player.maxs();

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

    Some((left as i32, top as i32, right as i32, bottom as i32))
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

pub fn is_ent_visible(from: &Vec3, to: &Vec3, ent: &Player, ignore_entity: &Player) -> bool {
    let trace = Interfaces::engine_trace().trace_ray(from, to, 0x4200400b, Some(ignore_entity));
    trace.fraction >= 0.97 || trace.entity == ent.this || trace.end_pos == *to
}
