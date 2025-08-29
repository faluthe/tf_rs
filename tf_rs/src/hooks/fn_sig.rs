use std::{ffi::c_void, fmt};

#[derive(Copy, Clone)]
pub enum FnSig {
    CreateMove(extern "C" fn(*mut c_void, f32, *mut c_void) -> i64),
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
