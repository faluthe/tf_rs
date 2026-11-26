use std::ffi::c_void;

use crate::{
    types::{MoveData, UserCmd},
    vfunc,
};

/// CPrediction
#[derive(Default, Clone)]
pub struct Prediction {
    this: *mut c_void,
}

impl Prediction {
    pub fn new(this: *mut c_void) -> Self {
        Prediction { this }
    }

    fn flag(&self, offset: usize) -> *mut bool {
        unsafe { (self.this as *mut u8).add(offset) as *mut bool }
    }

    pub fn in_prediction(&self) -> bool {
        unsafe { *self.flag(0xC) }
    }

    pub fn set_in_prediction(&self, value: bool) {
        unsafe { *self.flag(0xC) = value }
    }

    pub fn first_time_predicted(&self) -> bool {
        unsafe { *self.flag(0xD) }
    }

    pub fn set_first_time_predicted(&self, value: bool) {
        unsafe { *self.flag(0xD) = value }
    }

    pub fn engine_paused(&self) -> bool {
        unsafe { *self.flag(0xF) }
    }

    fn vtable(&self) -> *mut *mut c_void {
        unsafe { *(self.this as *mut *mut *mut c_void) }
    }

    /// Fill `data` from the player and command the way prediction does before running movement
    pub fn setup_move(
        &self,
        player: *mut c_void,
        cmd: *mut UserCmd,
        helper: *mut c_void,
        data: &mut MoveData,
    ) {
        let f = vfunc!(
            self.vtable(),
            19,
            extern "C" fn(*mut c_void, *mut c_void, *mut UserCmd, *mut c_void, *mut MoveData) -> ()
        );
        f(self.this, player, cmd, helper, data)
    }
}
