use std::ffi::c_void;

use crate::types::Player;

#[derive(Default, Clone)]
pub struct EntityList {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl EntityList {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        EntityList { this, vtable }
    }

    pub fn get_client_entity(&self, i: i32) -> Option<Player> {
        let func: extern "C" fn(*mut c_void, i32) -> *mut c_void =
            unsafe { std::mem::transmute(*self.vtable.add(3)) };

        let ptr = func(self.this, i);
        if ptr.is_null() {
            return None;
        }

        Some(Player::new(ptr))
    }
}
