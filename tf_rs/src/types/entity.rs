use std::ffi::c_void;

use crate::{
    offset_get,
    traits::FromRaw,
    types::{RGBA, Vec3, rgba},
    vfunc,
};

#[derive(PartialEq, Eq, Copy, Clone)]
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
    offset_get!(pub fn team: Team, 0xDC);
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

    pub fn class_id(&self) -> Option<ClassId> {
        let client_class = self.client_class();
        let i = unsafe { *((client_class as usize + 0x28) as *const i32) };
        match i {
            1 => Some(ClassId::AmmoHealth),
            86 => Some(ClassId::Dispenser),
            88 => Some(ClassId::Sentry),
            89 => Some(ClassId::Teleporter),
            112 => Some(ClassId::Arrow),
            217 => Some(ClassId::PillOrSticky),
            247 => Some(ClassId::Player),
            257 => Some(ClassId::Flare),
            258 => Some(ClassId::CrossbowBolt),
            264 => Some(ClassId::Rocket),
            _ => return None,
        }
    }

    pub fn is_dormant(&self) -> bool {
        let networkable = self.get_networkable();
        let vtable = unsafe { *(networkable as *mut *mut *mut c_void) };
        let f = vfunc!(vtable, 8, extern "C" fn(*mut c_void) -> bool);
        f(networkable)
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum ClassId {
    AmmoHealth = 1,
    Dispenser = 86,
    Sentry = 88,
    Teleporter = 89,
    Arrow = 112,
    Rocket = 264,
    PillOrSticky = 217,
    Player = 247,
    Flare = 257,
    CrossbowBolt = 258,
}

#[allow(dead_code)]
#[repr(i32)]
#[derive(PartialEq, Eq)]
pub enum Team {
    Spectator = 1,
    Red = 2,
    Blue = 3,
}

impl Team {
    pub fn as_rgba(&self) -> &'static RGBA {
        match self {
            Team::Red => &rgba::RED,
            Team::Blue => &rgba::BLUE,
            Team::Spectator => &rgba::WHITE,
        }
    }
}
