use std::ffi::{CStr, c_char, c_void};

use crate::{traits::FromRaw, vfunc};

pub const MATERIAL_VAR_NO_DRAW: i32 = 1 << 2;
pub const MATERIAL_VAR_IGNORE_Z: i32 = 1 << 15;

#[derive(Copy, Clone)]
pub struct Material {
    pub this: *mut c_void,
    pub vtable: *mut *mut c_void,
}

unsafe impl Send for Material {}
unsafe impl Sync for Material {}

impl FromRaw for Material {
    fn from_raw(raw: *mut c_void) -> Self {
        let vtable = unsafe { *(raw as *mut *mut *mut c_void) };
        Material { this: raw, vtable }
    }
}

impl Material {
    pub fn get_name(&self) -> &str {
        let f = vfunc!(self.vtable, 0, (*mut c_void) -> *const c_char);
        let ptr = f(self.this);
        if ptr.is_null() {
            return "";
        }
        unsafe { CStr::from_ptr(ptr).to_str().unwrap_or("") }
    }

    pub fn alpha_modulate(&self, a: f32) {
        let f = vfunc!(self.vtable, 27, (*mut c_void, f32) -> ());
        f(self.this, a);
    }

    pub fn color_modulate(&self, r: f32, g: f32, b: f32) {
        let f = vfunc!(self.vtable, 28, (*mut c_void, f32, f32, f32) -> ());
        f(self.this, r, g, b);
    }

    pub fn set_material_var_flags(&self, flag: i32, on: bool) {
        let f = vfunc!(self.vtable, 29, (*mut c_void, i32, bool) -> ());
        f(self.this, flag, on);
    }
}
