use std::ffi::{CString, c_char, c_void};

use crate::vfunc;

pub const MDL_HANDLE_INVALID: u16 = 0xFFFF;

/// Loads models by path, without the engine's model list, so they can be drawn outside a map
#[derive(Default, Clone)]
pub struct MdlCache {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl MdlCache {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        MdlCache { this, vtable }
    }

    pub fn is_valid(&self) -> bool {
        !self.this.is_null()
    }

    /// Handle of the model at `path`, loading it. Each call holds a reference
    pub fn find_mdl(&self, path: &str) -> Option<u16> {
        let f = vfunc!(self.vtable, 6, (*mut c_void, *const c_char) -> u16);
        let path = CString::new(path).ok()?;
        let handle = f(self.this, path.as_ptr());
        (handle != MDL_HANDLE_INVALID).then_some(handle)
    }

    /// The model file as loaded, a studiohdr_t
    pub fn studio_hdr(&self, handle: u16) -> *const u8 {
        let f = vfunc!(self.vtable, 10, (*mut c_void, u16) -> *const u8);
        f(self.this, handle)
    }

    /// What the studio renderer draws the model from
    pub fn hardware_data(&self, handle: u16) -> *mut c_void {
        let f = vfunc!(self.vtable, 11, (*mut c_void, u16) -> *mut c_void);
        f(self.this, handle)
    }

    pub fn is_error_model(&self, handle: u16) -> bool {
        let f = vfunc!(self.vtable, 20, (*mut c_void, u16) -> bool);
        f(self.this, handle)
    }
}
