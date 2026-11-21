use std::ffi::c_void;

use crate::vfunc;

#[allow(dead_code)]
#[repr(i32)]
pub enum Flow {
    Outgoing = 0,
    Incoming = 1,
}

/// The engine's CNetChan, of which INetChannelInfo is the primary base
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

    pub fn ptr(&self) -> *mut c_void {
        self.this
    }

    fn field(&self, offset: usize) -> *mut i32 {
        (self.this as usize + offset) as *mut i32
    }

    /// m_nInSequenceNr, the newest packet we received and acknowledge
    pub fn in_sequence(&self) -> i32 {
        unsafe { *self.field(0x10) }
    }

    pub fn set_in_sequence(&self, sequence: i32) {
        unsafe { *self.field(0x10) = sequence }
    }

    /// m_nInReliableState, flips with each reliable message received
    pub fn in_reliable_state(&self) -> i32 {
        unsafe { *self.field(0x1C) }
    }

    pub fn set_in_reliable_state(&self, state: i32) {
        unsafe { *self.field(0x1C) = state }
    }
}
