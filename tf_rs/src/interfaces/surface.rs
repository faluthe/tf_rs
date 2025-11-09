use std::ffi::c_void;

use crate::vfunc;

#[derive(Default, Clone)]
pub struct Surface {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl Surface {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        Surface { this, vtable }
    }

    pub fn draw_set_color(&self, r: i32, g: i32, b: i32, a: i32) {
        let f = vfunc!(
            self.vtable,
            10,
            extern "C" fn(*mut c_void, i32, i32, i32, i32) -> ()
        );
        f(self.this, r, g, b, a)
    }

    pub fn draw_outlined_rect(&self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let f = vfunc!(
            self.vtable,
            14,
            extern "C" fn(*mut c_void, i32, i32, i32, i32) -> ()
        );
        f(self.this, x0, y0, x1, y1)
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        let f = vfunc!(self.vtable, 52, extern "C" fn(*mut c_void, bool) -> ());
        f(self.this, visible)
    }
}
