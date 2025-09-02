use std::ffi::c_void;

use crate::{offset_get, types::Vec3, vfunc};

#[derive(PartialEq, Eq)]
pub struct Player {
    this: *mut c_void,
}

impl Player {
    pub fn new(this: *mut c_void) -> Self {
        Player { this }
    }

    offset_get!(pub fn health: i32, 0xD4);
    offset_get!(pub fn flags: i32, 0x460);
    offset_get!(pub fn team: i32, 0xDC);
    offset_get!(pub fn origin: Vec3, 0x328);
    offset_get!(fn lifestate: i8, 0x746);

    fn get_networkable(&self) -> *mut c_void {
        (self.this as usize + 0x10) as *mut c_void
    }

    fn get_collideable(&self) -> *mut c_void {
        (self.this as usize + 0x240) as *mut c_void
    }

    pub fn is_on_ground(&self) -> bool {
        (self.flags() & 1) == 0
    }

    pub fn is_dead(&self) -> bool {
        self.lifestate() != 1
    }

    pub fn is_dormant(&self) -> bool {
        let networkable = self.get_networkable();
        let vtable = unsafe { *(networkable as *mut *mut *mut c_void) };
        let f = vfunc!(vtable, 8, extern "C" fn(*mut c_void) -> bool);
        f(networkable)
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
}
