use std::{ffi::c_void, ops::Deref, ptr};

use crate::{
    interfaces::Interfaces,
    offset_get,
    traits::FromRaw,
    types::{Entity, Matrix3x4, Vec3, Weapon},
    vfunc,
};

pub const MAX_BONES: usize = 128;

pub const BONE_USED_BY_HITBOX: i32 = 0x100;
pub const BONE_USED_BY_ANYTHING: i32 = 0x7FF00;

/// Bone to world transforms, as set up by `setup_bones`
pub type Bones = [Matrix3x4; MAX_BONES];

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
    offset_get!(pub fn observer_mode: i32, 0x1644);
    offset_get!(pub fn fov: i32, 0x15E4);
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
    offset_get!(fn observer_target_: i32, 0x1648);
    offset_get!(pub fn eye_yaw: f32, 0x23F0);

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

    /// Server time of the command being built, from our tick base
    pub fn curtime(&self) -> f32 {
        Interfaces::global_vars().ticks_to_time(self.tick_base())
    }

    pub fn can_attack(&self) -> bool {
        let weapon = match self.active_weapon() {
            Some(weapon) => weapon,
            None => return false,
        };

        let next_attack = weapon.next_attack();

        if next_attack > self.curtime() {
            return false;
        }

        true
    }

    /// Fill `out` with the bone to world matrices of every bone in `mask`
    pub fn setup_bones(&self, out: &mut Bones, mask: i32, time: f32) -> bool {
        let f = vfunc!(
            self.vtable,
            96,
            extern "C" fn(*mut c_void, *mut Matrix3x4, i32, i32, f32) -> i32
        );
        f(self.this, out.as_mut_ptr(), MAX_BONES as i32, mask, time) != 0
    }

    /// Make the next setup_bones rebuild the bones instead of returning the ones cached this frame
    pub fn invalidate_bone_cache(&self) {
        unsafe {
            // m_iMostRecentModelBoneCounter, anything but the current g_iModelBoneCounter
            let counter = (self.this as *mut u8).add(0x820) as *mut u64;
            *counter = (*counter).wrapping_sub(1);
            // m_flLastBoneSetupTime
            *((self.this as *mut u8).add(0xBA0) as *mut f32) = -f32::MAX;
        }
    }

    /// Leave the values the last update set instead of interpolating towards them
    pub fn disable_interpolation(&self) {
        unsafe {
            // m_VarMap: entries of { u16 type, u16 m_bNeedsToInterpolate, data, watcher }
            let entries = *((self.this as usize + 0x28) as *const *mut u8);
            let interpolated = *((self.this as usize + 0x48) as *const i32);
            if entries.is_null() {
                return;
            }
            for i in 0..interpolated.max(0) as usize {
                *(entries.add(i * 0x18 + 2) as *mut u16) = 0;
            }
        }
    }

    pub fn get_bone_position(&self, bone_id: usize) -> Option<Vec3> {
        let mut bones = [Matrix3x4::default(); MAX_BONES];
        if !self.setup_bones(&mut bones, BONE_USED_BY_HITBOX, 0.0) {
            return None;
        }
        Some(bones[bone_id].position())
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

    pub fn is_soldier(&self) -> bool {
        self.player_class() == PlayerClass::Soldier
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

    pub fn observer_target(&self) -> Option<Entity> {
        let index = self.observer_target_() & 0xFFF;
        Interfaces::entity_list().get_client_entity::<Entity>(index)
    }
}

// Warns for unconstructed variants since we always use it through ffi
#[allow(dead_code)]
#[repr(i32)]
#[derive(PartialEq)]
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
