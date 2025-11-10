use std::ffi::c_void;

use crate::{offset_get, traits::FromRaw};

pub struct Weapon {
    this: *mut c_void,
}

impl FromRaw for Weapon {
    fn from_raw(raw: *mut c_void) -> Self {
        Weapon { this: raw }
    }
}

impl Weapon {
    offset_get!(pub fn next_attack: f32, 0xE94);
}
