use std::ffi::c_void;

use crate::vfunc;

#[derive(Default, Clone)]
pub struct RenderView {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl RenderView {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        RenderView { this, vtable }
    }

    pub fn set_blend(&self, blend: f32) {
        let f = vfunc!(self.vtable, 4, (*mut c_void, f32) -> ());
        f(self.this, blend);
    }

    pub fn get_blend(&self) -> f32 {
        let f = vfunc!(self.vtable, 5, (*mut c_void) -> f32);
        f(self.this)
    }

    pub fn set_color_modulation(&self, color: [f32; 3]) {
        let f = vfunc!(self.vtable, 6, (*mut c_void, *const f32) -> ());
        f(self.this, color.as_ptr());
    }

    pub fn get_color_modulation(&self) -> [f32; 3] {
        let f = vfunc!(self.vtable, 7, (*mut c_void, *mut f32) -> ());
        let mut color = [0.0; 3];
        f(self.this, color.as_mut_ptr());
        color
    }
}
