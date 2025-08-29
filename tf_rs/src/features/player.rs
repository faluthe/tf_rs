use std::ffi::c_void;

pub struct Player {
    this: *mut c_void,
}

impl Player {
    pub fn new(this: *mut c_void) -> Self {
        Player { this }
    }

    pub fn health(&self) -> i32 {
        unsafe { *((self.this as usize + 0xD4) as *const i32) }
    }
}
