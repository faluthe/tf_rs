use std::ffi::{CString, c_void};

use crate::{types::ConVar, vfunc};

#[derive(Default, Clone)]
pub struct Cvar {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl Cvar {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        Cvar { this, vtable }
    }

    pub fn find_var(&self, name: &str) -> Option<ConVar> {
        if self.this.is_null() {
            return None;
        }
        let name = CString::new(name).ok()?;
        let f = vfunc!(self.vtable, 12, (*mut c_void, *const i8) -> *mut c_void);
        ConVar::new(f(self.this, name.as_ptr()))
    }
}
