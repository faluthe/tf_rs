use std::ffi::c_void;

/// A console variable owned by the game, read straight from its fields
#[derive(Clone, Copy)]
pub struct ConVar {
    this: *mut c_void,
}

// Convars live for the whole session and are only read
unsafe impl Send for ConVar {}
unsafe impl Sync for ConVar {}

impl ConVar {
    pub fn new(this: *mut c_void) -> Option<Self> {
        (!this.is_null()).then_some(ConVar { this })
    }

    /// Raw value, without the clamping server bounded convars apply in GetFloat
    pub fn float(&self) -> f32 {
        unsafe {
            // Values are stored on the parent, which is the convar itself unless it was linked
            let parent = *((self.this as usize + 0x38) as *const *mut c_void);
            let base = if parent.is_null() { self.this } else { parent };
            *((base as usize + 0x54) as *const f32)
        }
    }
}
