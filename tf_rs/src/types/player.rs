use std::ffi::c_void;

use crate::{
    interfaces::Interfaces,
    offset_get,
    traits::FromRaw,
    types::{Vec3, Weapon},
    vfunc,
};

#[derive(PartialEq, Eq)]
pub struct Player {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl FromRaw for Player {
    fn from_raw(raw: *mut c_void) -> Self {
        let vtable = unsafe { *(raw as *mut *mut *mut c_void) };
        Player { this: raw, vtable }
    }
}

impl Player {
    offset_get!(pub fn _health: i32, 0xD4);
    offset_get!(pub fn flags: i32, 0x460);
    offset_get!(pub fn team: i32, 0xDC);
    offset_get!(pub fn origin: Vec3, 0x328);
    offset_get!(fn lifestate: i8, 0x746);
    offset_get!(fn active_weapon_: i32, 0x11D0);
    offset_get!(fn tick_base: i32, 0x1718);
    offset_get!(fn eye_z_diff: f32, 0x14C);

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

    pub fn active_weapon(&self) -> i32 {
        // lower 12 bits represent the index
        self.active_weapon_() & 0xFFF
    }

    pub fn eye_pos(&self) -> Vec3 {
        let mut eye_pos = self.origin();
        eye_pos.z += self.eye_z_diff();
        eye_pos
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

    pub fn can_attack(&self) -> bool {
        let Some(weapon) =
            Interfaces::entity_list().get_client_entity::<Weapon>(self.active_weapon())
        else {
            return false;
        };

        let next_attack = weapon.next_attack();
        let cur_time = self.tick_base() as f32 * Interfaces::global_vars().interval_per_tick;

        if next_attack > cur_time {
            return false;
        }

        true
    }

    pub fn get_bone_position(&self, bone_id: usize) -> Option<Vec3> {
        let mut bone_to_world_out: [[[f32; 4]; 3]; 128] = [[[0.0; 4]; 3]; 128];
        let f = vfunc!(
            self.vtable,
            96,
            extern "C" fn(*mut c_void, *mut [[[f32; 4]; 3]; 128], i32, i32, f32) -> i32
        );
        if f(self.this, &mut bone_to_world_out, 128, 0x100, 0.0) == 0 {
            return None;
        }
        Some(Vec3 {
            x: bone_to_world_out[bone_id][0][3],
            y: bone_to_world_out[bone_id][1][3],
            z: bone_to_world_out[bone_id][2][3],
        })
    }
}
