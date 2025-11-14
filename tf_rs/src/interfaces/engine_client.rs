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

    pub fn get_screen_size(&self) -> (i32, i32) {
        let mut width = 0;
        let mut height = 0;
        let f = vfunc!(
            self.vtable,
            5,
            extern "C" fn(*mut c_void, &mut i32, &mut i32) -> ()
        );
        f(self.this, &mut width, &mut height);
        (width, height)
    }

    pub fn get_localplayer_index(&self) -> i32 {
        let f = vfunc!(self.vtable, 12, extern "C" fn(*mut c_void) -> i32);
        f(self.this)
    }

    pub fn is_in_game(&self) -> bool {
        let f = vfunc!(self.vtable, 26, extern "C" fn(*mut c_void) -> bool);
        f(self.this)
    }

    pub fn get_max_clients(&self) -> i32 {
        let f = vfunc!(self.vtable, 21, extern "C" fn(*mut c_void) -> i32);
        f(self.this)
    }
}
