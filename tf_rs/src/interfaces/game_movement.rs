use std::ffi::c_void;

use crate::{types::MoveData, vfunc};

#[derive(Default, Clone)]
pub struct GameMovement {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl GameMovement {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        GameMovement { this, vtable }
    }

    pub fn process_movement(&self, player: *mut c_void, data: &mut MoveData) {
        let f = vfunc!(
            self.vtable,
            2,
            extern "C" fn(*mut c_void, *mut c_void, *mut MoveData) -> ()
        );
        f(self.this, player, data)
    }
}
