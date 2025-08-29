use std::ffi::c_void;

use crate::types::Vec3;

#[repr(C)]
pub struct UserCmd {
    pub something: *mut c_void,
    pub command_number: i32,
    pub tick_count: i32,
    pub view_angles: Vec3,
    pub forward_move: f32,
    pub side_move: f32,
    pub up_move: f32,
    pub buttons: i32,
    pub impulse: u8,
    pub weapon_select: i32,
    pub weapon_subtype: i32,
    pub random_seed: i32,
    pub mouse_dx: i16,
    pub mouse_dy: i16,
    pub has_been_predicted: bool,
}

#[repr(i32)]
pub enum Buttons {
    InJump = 1 << 1,
}
