use std::{ffi::c_void, ops::Deref, ptr};

use crate::{
    interfaces::Interfaces,
    offset_get,
    traits::FromRaw,
    types::{Entity, Vec3, Weapon, WeaponClass},
    vfunc,
};

#[derive(PartialEq, Eq)]
pub struct Player {
    pub ent: Entity,
}

impl FromRaw for Player {
    fn from_raw(raw: *mut c_void) -> Self {
        let vtable = unsafe { *(raw as *mut *mut *mut c_void) };
        Player {
            ent: Entity { this: raw, vtable },
        }
    }
}

impl Deref for Player {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.ent
    }
}

impl Player {
    offset_get!(pub fn flags: i32, 0x460);
    offset_get!(fn lifestate: i8, 0x746);
    offset_get!(fn active_weapon_: i32, 0x11D0);
    offset_get!(fn tick_base: i32, 0x1718);
    offset_get!(fn eye_z_diff: f32, 0x14C);
    offset_get!(fn player_class: PlayerClass, 0x1BA0);
    offset_get!(fn conds_0: u32, 0x1F64);
    offset_get!(fn conds_1: u32, 0x1F68);
    offset_get!(fn conds_2: u32, 0x1F6C);
    offset_get!(fn conds_3: u32, 0x1F70);
    offset_get!(fn conds_4: u32, 0x1F74);
    offset_get!(fn invisibility: f32, 0x1FE8);

    pub fn is_on_ground(&self) -> bool {
        (self.flags() & 1) != 0
    }

    pub fn is_dead(&self) -> bool {
        self.lifestate() != 1
    }

    pub fn active_weapon(&self) -> Option<Weapon> {
        // lower 12 bits represent the index
        let index = self.active_weapon_() & 0xFFF;
        Interfaces::entity_list().get_client_entity::<Weapon>(index)
    }

    pub fn eye_pos(&self) -> Vec3 {
        let mut eye_pos = self.origin();
        eye_pos.z += self.eye_z_diff();
        eye_pos
    }

    pub fn can_attack(&self) -> bool {
        let weapon = match self.active_weapon() {
            Some(weapon) => weapon,
            None => return false,
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

    pub fn can_headshot(&self) -> bool {
        let weapon = match self.active_weapon() {
            Some(weapon) => weapon,
            None => return false,
        };

        match (self.player_class(), weapon.weapon_class()) {
            (PlayerClass::Sniper, WeaponClass::Sniperrifle) => true,
            _ => false,
        }
    }

    pub fn head_bone_id(&self) -> usize {
        match self.player_class() {
            PlayerClass::Engineer => 8,
            PlayerClass::Demoman => 16,
            PlayerClass::Sniper | PlayerClass::Soldier => 5,
            _ => 6,
        }
    }

    pub fn in_cond(&self, cond: Cond) -> bool {
        let cond = cond as u32;
        let group = cond / 32;

        let bit = 1u32 << (cond % 32);

        match group {
            0 => (self.conds_0() & bit) != 0,
            1 => (self.conds_1() & bit) != 0,
            2 => (self.conds_2() & bit) != 0,
            3 => (self.conds_3() & bit) != 0,
            4 => (self.conds_4() & bit) != 0,
            _ => false,
        }
    }

    pub fn is_invisible(&self) -> bool {
        if self.in_cond(Cond::Burning) || self.in_cond(Cond::MadMilk) || self.in_cond(Cond::Urine) {
            return false;
        }

        self.invisibility() > 0.9
    }

    pub fn set_thirdperson(&self, enabled: i32) {
        unsafe {
            let p = (self.this as *const u8).add(0x240C as usize) as *mut i32;
            ptr::write(p, enabled);
        }
    }
}

// Warns for unconstructed variants since we always use it through ffi
#[allow(dead_code)]
#[repr(i32)]
enum PlayerClass {
    Undefined = 0,
    Scout = 1,
    Sniper = 2,
    Soldier = 3,
    Demoman = 4,
    Medic = 5,
    HeavyWeapons = 6,
    Pyro = 7,
    Spy = 8,
    Engineer = 9,
}

#[allow(dead_code)]
#[repr(u32)]
pub enum Cond {
    Zoomed = 1,
    Disguised = 3,
    Taunting = 7,
    Burning = 22,
    Urine = 24,
    MadMilk = 27,
}
