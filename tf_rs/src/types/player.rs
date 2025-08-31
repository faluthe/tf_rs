use std::ffi::c_void;

use crate::offset_get;

pub struct Player {
    this: *mut c_void,
}

impl Player {
    pub fn new(this: *mut c_void) -> Self {
        Player { this }
    }

    offset_get!(pub fn health: i32, 0xD4);
    offset_get!(pub fn flags: i32, 0x460);

    pub fn is_on_ground(&self) -> bool {
        (self.flags() & 1) == 0
    }
}
