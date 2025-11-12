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
}
