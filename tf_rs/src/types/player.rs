use std::{ffi::c_void, ops::Deref, ptr};

use crate::{
    interfaces::Interfaces,
    offset_get,
    traits::FromRaw,
    types::{Entity, Matrix3x4, Vec3, Weapon},
    vfunc,
};

pub const MAX_BONES: usize = 128;
pub const FL_ONGROUND: i32 = 1 << 0;
pub const FL_DUCKING: i32 = 1 << 1;
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
    offset_get!(pub fn tick_base: i32, 0x1718);
    offset_get!(fn next_attack: f32, 0x1088);
    offset_get!(fn eye_z_diff: f32, 0x14C);
    offset_get!(pub fn player_class: PlayerClass, 0x1BA0);
    offset_get!(fn conds_0: u32, 0x1F64);
    offset_get!(fn conds_1: u32, 0x1F68);
    offset_get!(fn conds_2: u32, 0x1F6C);
    offset_get!(fn conds_3: u32, 0x1F70);
    offset_get!(fn conds_4: u32, 0x1F74);
    offset_get!(fn invisibility: f32, 0x1FE8);
    offset_get!(fn observer_target_: i32, 0x1648);
    offset_get!(pub fn eye_yaw: f32, 0x23F0);
    offset_get!(pub fn eye_pitch: f32, 0x23EC);
    offset_get!(pub fn view_offset: Vec3, 0x144);
    offset_get!(pub fn max_speed: f32, 0x1680);
    offset_get!(pub fn water_level: u8, 0x218);
    offset_get!(pub fn ducked: bool, 0x1288);
    offset_get!(pub fn constraint_entity: u32, 0x1628);
    offset_get!(pub fn constraint_center: Vec3, 0x162C);
    offset_get!(pub fn constraint_radius: f32, 0x1638);
    offset_get!(pub fn constraint_width: f32, 0x163C);
    offset_get!(pub fn constraint_speed_factor: f32, 0x1640);

    pub fn is_on_ground(&self) -> bool {
        (self.flags() & FL_ONGROUND) != 0
    }

    pub fn is_ducking(&self) -> bool {
        (self.flags() & FL_DUCKING) != 0
    }

    pub fn is_swimming(&self) -> bool {
        self.water_level() >= 2
    }

    pub fn set_flags(&self, flags: i32) {
        unsafe { *self.field::<i32>(0x460) = flags }
    }

    pub fn set_max_speed(&self, speed: f32) {
        unsafe { *self.ent.field::<f32>(0x1680) = speed };
    }

    pub fn set_velocity(&self, velocity: Vec3) {
        unsafe { *self.field::<Vec3>(0x168) = velocity }
    }

    pub fn set_base_velocity(&self, velocity: Vec3) {
        unsafe { *self.field::<Vec3>(0x1F0) = velocity }
    }

    pub fn clear_ground_entity(&self) {
        unsafe { *self.field::<u32>(0x31C) = u32::MAX }
    }

    pub fn set_ducked(&self, ducked: bool) {
        unsafe { *self.field::<bool>(0x1288) = ducked }
    }

    /// Clear m_bDucking, m_bInDuckJump, m_flDucktime and m_flDuckJumpTime
    pub fn reset_duck_transition(&self) {
        unsafe {
            *self.field::<bool>(0x1289) = false;
            *self.field::<bool>(0x128A) = false;
            *self.field::<f32>(0x128C) = 0.0;
            *self.field::<f32>(0x1290) = 0.0;
        }
    }

    /// m_pCurrentCommand, what game movement reads the command being run from
    pub fn set_current_command(&self, cmd: *mut c_void) {
        unsafe { *self.field::<*mut c_void>(0x1620) = cmd }
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

        // The weapon's own delay, and the player's, which reloads and switching weapons set
        let cur_time = self.curtime();
        weapon.next_attack() <= cur_time && self.next_attack() <= cur_time
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
#[derive(PartialEq, Clone, Copy)]
pub enum PlayerClass {
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
    ShieldCharge = 17,
    Burning = 22,
    Urine = 24,
    MadMilk = 27,
}
