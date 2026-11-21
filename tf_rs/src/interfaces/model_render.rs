use std::{ffi::c_void, ptr};

use crate::{types::Material, vfunc};

#[derive(Default, Clone)]
pub struct ModelRender {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl ModelRender {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        ModelRender { this, vtable }
    }

    pub fn ptr(&self) -> *mut c_void {
        self.this
    }

    pub fn forced_material_override(&self, material: Option<&Material>) {
        let f = vfunc!(self.vtable, 1, (*mut c_void, *mut c_void, i32) -> ());
        f(self.this, material.map_or(ptr::null_mut(), |m| m.this), 0);
    }
}
