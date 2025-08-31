use anyhow::{Result, anyhow};
use std::{ffi::c_void, fmt, mem};

use crate::types::UserCmd;

#[derive(Copy, Clone)]
pub enum FnSig {
    CreateMove(extern "C" fn(*mut c_void, f32, *mut UserCmd) -> i64),
    PaintTraverse(extern "C" fn(*mut c_void, *mut c_void, i8, i8) -> i64),
    None,
}

impl fmt::Pointer for FnSig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FnSig::CreateMove(func) => write!(f, "{:p}", *func as *const c_void),
            FnSig::PaintTraverse(func) => write!(f, "{:p}", *func as *const c_void),
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
    ) -> Result<i64> {
        match self {
            FnSig::CreateMove(f) => Ok(f(this, sample_time, cmd)),
            _ => Err(anyhow!("Invalid CreateMove function signature")),
        }
    }

    pub fn call_paint_traverse(
        &self,
        this: *mut c_void,
        panel: *mut c_void,
        force_repaint: i8,
        allow_force: i8,
    ) -> Result<i64> {
        match self {
            FnSig::PaintTraverse(f) => Ok(f(this, panel, force_repaint, allow_force)),
            _ => Err(anyhow!("Invalid PaintTraverse function signature")),
        }
    }

    pub fn from_ptr(ptr: *mut c_void, signature: Self) -> Self {
        match signature {
            FnSig::CreateMove(_) => FnSig::CreateMove(unsafe {
                mem::transmute::<*mut c_void, extern "C" fn(*mut c_void, f32, *mut UserCmd) -> i64>(
                    ptr,
                )
            }),
            FnSig::PaintTraverse(_) => FnSig::PaintTraverse(unsafe {
                mem::transmute::<*mut c_void, extern "C" fn(*mut c_void, *mut c_void, i8, i8) -> i64>(
                    ptr,
                )
            }),
            FnSig::None => FnSig::None,
        }
    }

    pub fn as_ptr(&self) -> Result<*mut c_void> {
        match self {
            FnSig::CreateMove(f) => Ok(*f as *mut c_void),
            FnSig::PaintTraverse(f) => Ok(*f as *mut c_void),
            FnSig::None => Err(anyhow!("FnSig is None")),
        }
    }
}
