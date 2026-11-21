use crate::types::{Vec3, Vec3Aligned};

#[repr(C)]
pub struct Ray {
    _start: Vec3Aligned,
    _delta: Vec3Aligned,
    _start_offset: Vec3Aligned,
    _extents: Vec3Aligned,
    _is_ray: bool,
    _is_swept: bool,
}

impl Ray {
    pub fn init(from: &Vec3, to: &Vec3) -> Self {
        let delta = Vec3Aligned {
            x: to.x - from.x,
            y: to.y - from.y,
            z: to.z - from.z,
        };

        let is_swept = delta.x != 0.0 || delta.y != 0.0 || delta.z != 0.0;

        Ray {
            _start: Vec3Aligned {
                x: from.x,
                y: from.y,
                z: from.z,
            },
            _delta: delta,
            _start_offset: Vec3Aligned {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            _extents: Vec3Aligned {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            _is_ray: true,
            _is_swept: is_swept,
        }
    }

    /// A box with bounds `mins`..`maxs` swept from `from` to `to`
    pub fn hull(from: &Vec3, to: &Vec3, mins: &Vec3, maxs: &Vec3) -> Self {
        let mut ray = Ray::init(from, to);
        let half = |a: f32, b: f32| (b - a) * 0.5;
        let center = |a: f32, b: f32| (a + b) * 0.5;
        ray._extents = Vec3Aligned {
            x: half(mins.x, maxs.x),
            y: half(mins.y, maxs.y),
            z: half(mins.z, maxs.z),
        };
        ray._is_ray = ray._extents.x * ray._extents.x
            + ray._extents.y * ray._extents.y
            + ray._extents.z * ray._extents.z
            < 1e-6;
        // The engine traces the box's center, offset back to `from` in the results
        let offset = Vec3Aligned {
            x: center(mins.x, maxs.x),
            y: center(mins.y, maxs.y),
            z: center(mins.z, maxs.z),
        };
        ray._start.x += offset.x;
        ray._start.y += offset.y;
        ray._start.z += offset.z;
        ray._start_offset = Vec3Aligned {
            x: -offset.x,
            y: -offset.y,
            z: -offset.z,
        };
        ray
    }
}
