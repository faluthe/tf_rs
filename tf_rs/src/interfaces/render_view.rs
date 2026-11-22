use std::{ffi::c_void, ptr};

use crate::{
    types::view_setup::{Frustum, ViewSetup},
    vfunc,
};

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

    /// Draw into `view` until `pop_view`, which takes the same `frustum`
    pub fn push_3d_view(&self, view: &ViewSetup, clear_flags: i32, frustum: &mut Frustum) {
        let f = vfunc!(
            self.vtable,
            37,
            (*mut c_void, *const ViewSetup, i32, *mut c_void, *mut [f32; 4]) -> ()
        );
        f(
            self.this,
            view,
            clear_flags,
            ptr::null_mut(),
            frustum.as_mut_ptr(),
        );
    }

    pub fn pop_view(&self, frustum: &mut Frustum) {
        let f = vfunc!(self.vtable, 39, (*mut c_void, *mut [f32; 4]) -> ());
        f(self.this, frustum.as_mut_ptr());
    }

    pub fn get_color_modulation(&self) -> [f32; 3] {
        let f = vfunc!(self.vtable, 7, (*mut c_void, *mut f32) -> ());
        let mut color = [0.0; 3];
        f(self.this, color.as_mut_ptr());
        color
    }
}
