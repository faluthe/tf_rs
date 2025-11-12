use std::ffi::c_void;

use crate::types::Vec3;

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
