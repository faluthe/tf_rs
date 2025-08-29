use std::{ffi::c_void, fmt};

use crate::types::UserCmd;

#[derive(Copy, Clone)]
pub enum FnSig {
    CreateMove(extern "C" fn(*mut c_void, f32, *mut UserCmd) -> i64),
    None,
}

impl fmt::Pointer for FnSig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FnSig::CreateMove(func) => write!(f, "{:p}", *func as *const c_void),
            FnSig::None => write!(f, "None"),
        }
    }
}

impl FnSig {
    pub fn call_create_move(
        &self,
        this: *mut c_void,
        sample_time: f32,
        cmd: *mut UserCmd,
    ) -> Option<i64> {
        match self {
            FnSig::CreateMove(f) => Some(f(this, sample_time, cmd)),
            _ => None,
        }
    }
}
