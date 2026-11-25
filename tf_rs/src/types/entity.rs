use std::ffi::c_void;

use crate::{
    offset_get,
    traits::FromRaw,
    types::{Color, Hull, Vec3, rgba},
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
    offset_get!(pub fn simulation_time: f32, 0x98);
    offset_get!(pub fn velocity: Vec3, 0x168);

    pub fn index(&self) -> i32 {
        let networkable = self.get_networkable();
        let vtable = unsafe { *(networkable as *mut *mut *mut c_void) };
        let f = vfunc!(vtable, 9, extern "C" fn(*mut c_void) -> i32);
        f(networkable)
    }

    /// m_vecOrigin as last networked, without interpolation
    pub fn network_origin(&self) -> Vec3 {
        unsafe { *self.field::<Vec3>(0x448) }
    }

    /// Pointer to the member at `offset`, for fields that get written
    pub fn field<T>(&self, offset: usize) -> *mut T {
        (self.this as usize + offset) as *mut T
    }

    /// Prediction datamap, the fields prediction saves and restores
    pub fn pred_desc_map(&self) -> *const c_void {
        let f = vfunc!(self.vtable, 18, extern "C" fn(*mut c_void) -> *const c_void);
        f(self.this)
    }

    /// The entity's own handle
    pub fn ref_handle(&self) -> u32 {
        let f = vfunc!(self.vtable, 3, extern "C" fn(*mut c_void) -> *const u32);
        unsafe { *f(self.this) }
    }

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

    /// Collision box at the entity's origin
    pub fn hull(&self) -> Hull {
        Hull {
            origin: self.origin(),
            mins: self.mins(),
            maxs: self.maxs(),
        }
    }

    /// Network name of the entity's client class, e.g. "CTFViewModel"
    pub fn class_name(&self) -> &str {
        let client_class = self.client_class();
        if client_class.is_null() {
            return "";
        }
        unsafe {
            let name = *((client_class as usize + 0x10) as *const *const std::ffi::c_char);
            if name.is_null() {
                return "";
            }
            std::ffi::CStr::from_ptr(name).to_str().unwrap_or("")
        }
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
#[derive(PartialEq, Eq, Debug)]
pub enum Team {
    Spectator = 1,
    Red = 2,
    Blue = 3,
}

impl Team {
    pub fn as_color(&self) -> &'static Color {
        match self {
            Team::Red => &rgba::RED,
            Team::Blue => &rgba::BLUE,
            Team::Spectator => &rgba::WHITE,
        }
    }
}
