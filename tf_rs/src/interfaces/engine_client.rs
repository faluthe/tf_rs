use std::ffi::c_void;

use crate::vfunc;

#[derive(Default, Clone)]
pub struct EngineClient {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl EngineClient {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        EngineClient { this, vtable }
    }

    pub fn get_localplayer_index(&self) -> i32 {
        let f = vfunc!(self.vtable, 12, extern "C" fn(*mut c_void) -> i32);
        f(self.this)
    }
}
