use std::ffi::c_void;

use crate::{
    types::{Vec2, Vec3},
    vfunc,
};

#[derive(Default, Clone)]
pub struct DebugOverlay {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl DebugOverlay {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        DebugOverlay { this, vtable }
    }

    pub fn screen_position(&self, point: &Vec3) -> Option<Vec2> {
        let mut out = Vec3::new(0.0, 0.0, 0.0);
        let f = vfunc!(
            self.vtable,
            9,
            extern "C" fn(*mut c_void, *const Vec3, *mut Vec3) -> i32
        );
        if f(self.this, point as *const Vec3, &mut out as *mut Vec3) == 0 {
            Some(Vec2::new(out.x, out.y))
        } else {
            None
        }
    }
}
