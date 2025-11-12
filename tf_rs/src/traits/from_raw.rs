use std::ffi::c_void;

pub trait FromRaw: Sized {
    fn from_raw(raw: *mut c_void) -> Self;
}
