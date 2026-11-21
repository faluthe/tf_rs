use std::ffi::{CStr, c_char, c_void};

use crate::vfunc;

#[derive(Default, Clone)]
pub struct ModelInfo {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl ModelInfo {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        ModelInfo { this, vtable }
    }

    pub fn get_model_name(&self, model: *const c_void) -> &str {
        let f = vfunc!(self.vtable, 4, (*mut c_void, *const c_void) -> *const c_char);
        let ptr = f(self.this, model);
        if ptr.is_null() {
            return "";
        }
        unsafe { CStr::from_ptr(ptr).to_str().unwrap_or("") }
    }
}
