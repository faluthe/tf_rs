use std::ffi::c_void;

use crate::vfunc;

#[allow(dead_code)]
#[repr(i32)]
pub enum Flow {
    Outgoing = 0,
    Incoming = 1,
}

pub struct NetChannelInfo {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl NetChannelInfo {
    pub fn new(this: *mut c_void) -> Option<Self> {
        if this.is_null() {
            return None;
        }
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        Some(NetChannelInfo { this, vtable })
    }

    /// Current latency of `flow` in seconds
    pub fn latency(&self, flow: Flow) -> f32 {
        let f = vfunc!(self.vtable, 9, (*mut c_void, i32) -> f32);
        f(self.this, flow as i32)
    }
}
