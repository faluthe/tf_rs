use std::ffi::{CStr, c_void};

use crate::vfunc;

#[derive(Default, Clone)]
pub struct Panel {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl Panel {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        Panel { this, vtable }
    }

    pub fn ptr(&self) -> *mut c_void {
        self.this
    }

    pub fn panel_name(&self, panel: *mut c_void) -> &str {
        let f = vfunc!(
            self.vtable,
            37,
            extern "C" fn(*mut c_void, *mut c_void) -> *const i8
        );
        let ptr = f(self.this, panel);
        unsafe { CStr::from_ptr(ptr).to_str().unwrap_or("") }
    }
}
