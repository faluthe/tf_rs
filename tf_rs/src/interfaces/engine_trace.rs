use std::ffi::c_void;

use crate::{
    types::{Player, Vec3},
    vfunc,
};

#[derive(Default, Clone)]
pub struct EngineTrace {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl EngineTrace {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        EngineTrace { this, vtable }
    }

    pub fn trace_ray(
        &self,
        from: &Vec3,
        to: &Vec3,
        mask: u32,
        ignore: Option<&Player>,
    ) -> TraceResult {
        let ray = Ray::init(from, to);
        let filter = TraceFilter::init(ignore);
        let mut result = TraceResult::default();

        let f = vfunc!(
            self.vtable,
            4,
            extern "C" fn(*mut c_void, *mut c_void, u32, *mut c_void, *mut TraceResult) -> ()
        );

        f(
            self.this,
            &ray as *const _ as *mut c_void,
            mask,
            &filter as *const _ as *mut c_void,
            &mut result,
        );
        result
    }
}

// TODO: move these lol
#[repr(C, align(16))]
pub struct VectorAligned {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
struct Ray {
    _start: VectorAligned,
    _delta: VectorAligned,
    _start_offset: VectorAligned,
    _extents: VectorAligned,
    _is_ray: bool,
    _is_swept: bool,
}

impl Ray {
    fn init(from: &Vec3, to: &Vec3) -> Self {
        let delta = VectorAligned {
            x: to.x - from.x,
            y: to.y - from.y,
            z: to.z - from.z,
        };

        let is_swept = delta.x != 0.0 || delta.y != 0.0 || delta.z != 0.0;

        Ray {
            _start: VectorAligned {
                x: from.x,
                y: from.y,
                z: from.z,
            },
            _delta: delta,
            _start_offset: VectorAligned {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            _extents: VectorAligned {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            _is_ray: true,
            _is_swept: is_swept,
        }
    }
}

#[repr(C)]
struct TraceFilter {
    vtable: *mut *mut c_void,
    skip: *mut Player,
}

#[repr(C)]
struct TraceFilterVTable {
    should_hit_entity: extern "C" fn(*const TraceFilter, *mut Player, i32) -> bool,
    get_trace_type: extern "C" fn(*const TraceFilter) -> i32,
}

static TRACE_FILTER_VTABLE: TraceFilterVTable = TraceFilterVTable {
    should_hit_entity: TraceFilter::should_hit_entity,
    get_trace_type: TraceFilter::get_trace_type,
};

impl TraceFilter {
    fn init(ignore: Option<&Player>) -> Self {
        TraceFilter {
            vtable: &TRACE_FILTER_VTABLE as *const _ as *mut *mut c_void,
            skip: match ignore {
                Some(player) => player as *const Player as *mut Player,
                None => std::ptr::null_mut(),
            },
        }
    }

    extern "C" fn should_hit_entity(filter: *const Self, entity: *mut Player, _mask: i32) -> bool {
        entity != unsafe { (*filter).skip }
    }

    extern "C" fn get_trace_type(_filter: *const Self) -> i32 {
        0 // TRACE_EVERYTHING
    }
}

#[repr(C)]
#[derive(Default)]
pub struct TraceResult {
    _start_pos: Vec3,
    pub end_pos: Vec3,
    _plane: CPlane,
    pub fraction: f32,
    _contents: i32,
    _disp_flags: u16,
    _all_solid: bool,
    _start_solid: bool,
    _fraction_left_solid: f32,
    _surface: CSurface,
    _hit_group: i32,
    _physics_bone: i16,
    pub entity: *mut c_void,
    _hitbox: i32,
}

#[repr(C)]
#[derive(Default)]
struct CPlane {
    _normal: Vec3,
    _dist: f32,
    _type: u8,
    _sign_bits: u8,
    _pad: [u8; 2],
}

#[repr(C)]
#[derive(Default)]
struct CSurface {
    _name: *const i8,
    _surface_props: i16,
    _flags: u16,
}
