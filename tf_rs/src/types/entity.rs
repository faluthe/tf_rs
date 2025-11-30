use std::ffi::c_void;

use crate::{offset_get, traits::FromRaw, types::Vec3, vfunc};

#[derive(PartialEq, Eq)]
pub struct Entity {
    pub this: *mut c_void,
    pub vtable: *mut *mut c_void,
}

impl FromRaw for Entity {
    fn from_raw(raw: *mut c_void) -> Self {
        let vtable = unsafe { *(raw as *mut *mut *mut c_void) };
        Entity { this: raw, vtable }
    }
}

impl Entity {
    offset_get!(pub fn team: i32, 0xDC);
    offset_get!(pub fn origin: Vec3, 0x328);

    fn get_networkable(&self) -> *mut c_void {
        (self.this as usize + 0x10) as *mut c_void
    }

    fn get_collideable(&self) -> *mut c_void {
        (self.this as usize + 0x240) as *mut c_void
    }

    fn client_class(&self) -> *mut c_void {
        let networkable = self.get_networkable();
        let vtable = unsafe { *(networkable as *mut *mut *mut c_void) };
        let f = vfunc!(vtable, 2, extern "C" fn(*mut c_void) -> *mut c_void);
        f(networkable)
    }

    pub fn health(&self) -> i32 {
        let f = vfunc!(self.vtable, 152, extern "C" fn(*mut c_void) -> i32);
        f(self.this)
    }

    pub fn max_health(&self) -> i32 {
        let f = vfunc!(self.vtable, 153, extern "C" fn(*mut c_void) -> i32);
        f(self.this)
    }

    pub fn mins(&self) -> Vec3 {
        let collideable = self.get_collideable();
        let vtable = unsafe { *(collideable as *mut *mut *mut c_void) };
        let f = vfunc!(vtable, 1, extern "C" fn(*mut c_void) -> *const Vec3);
        unsafe { *(f(collideable)) }
    }

    pub fn maxs(&self) -> Vec3 {
        let collideable = self.get_collideable();
        let vtable = unsafe { *(collideable as *mut *mut *mut c_void) };
        let f = vfunc!(vtable, 2, extern "C" fn(*mut c_void) -> *const Vec3);
        unsafe { *(f(collideable)) }
    }

    pub fn class_id(&self) -> EntityClassID {
        let client_class = self.client_class();
        unsafe { *((client_class as usize + 0x28) as *const EntityClassID) }
    }

    pub fn is_dormant(&self) -> bool {
        let networkable = self.get_networkable();
        let vtable = unsafe { *(networkable as *mut *mut *mut c_void) };
        let f = vfunc!(vtable, 8, extern "C" fn(*mut c_void) -> bool);
        f(networkable)
    }
}

#[allow(dead_code)]
#[repr(i32)]
#[derive(Copy, Clone)]
pub enum EntityClassID {
    AmmoHealth = 1,
    Dispenser = 86,
    Sentry = 88,
    Teleporter = 89,
    Arrow = 112,
    Rocket = 264,
    PillOrSticky = 217,
    Flare = 257,
    CrossbowBolt = 258,
}
