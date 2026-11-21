use std::ffi::c_void;

use crate::types::{Matrix3x4, Vec3};

#[repr(C)]
pub struct ModelRenderInfo {
    pub origin: Vec3,
    pub angles: Vec3,
    pub renderable: *mut c_void,
    pub model: *const c_void,
    pub model_to_world: *const Matrix3x4,
    pub lighting_offset: *const Matrix3x4,
    pub lighting_origin: *const Vec3,
    pub flags: i32,
    pub entity_index: i32,
    pub skin: i32,
    pub body: i32,
    pub hitboxset: i32,
    pub instance: u16,
}
