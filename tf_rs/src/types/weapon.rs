use std::ffi::c_void;

use crate::{offset_get, traits::FromRaw, types::Vec3, vfunc};

pub struct Weapon {
    pub this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl FromRaw for Weapon {
    fn from_raw(raw: *mut c_void) -> Self {
        let vtable = unsafe { *(raw as *mut *mut *mut c_void) };
        Weapon { this: raw, vtable }
    }
}

impl Weapon {
    offset_get!(pub fn next_attack: f32, 0xE94);

    pub fn weapon_class(&self) -> WeaponClass {
        let f = vfunc!(self.vtable, 451, extern "C" fn(*mut c_void) -> WeaponClass);
        f(self.this)
    }

    pub fn can_headshot(&self) -> bool {
        matches!(
            self.weapon_class(),
            WeaponClass::Sniperrifle | WeaponClass::Revolver
        )
    }

    pub fn spread(&self) -> f32 {
        let f = vfunc!(self.vtable, 540, extern "C" fn(*mut c_void) -> f32);
        f(self.this)
    }

    pub fn is_hitscan(&self) -> bool {
        matches!(
            self.weapon_class(),
            WeaponClass::ShotgunPrimary
                | WeaponClass::ShotgunSoldier
                | WeaponClass::ShotgunHwg
                | WeaponClass::ShotgunPyro
                | WeaponClass::Scattergun
                | WeaponClass::Sniperrifle
                | WeaponClass::Smg
                | WeaponClass::SyringegunMedic
                | WeaponClass::Tranq
                | WeaponClass::Pistol
                | WeaponClass::PistolScout
                | WeaponClass::Revolver
                | WeaponClass::Nailgun
                | WeaponClass::HandgunScoutPrimary
                | WeaponClass::HandgunScoutSecondary
                | WeaponClass::SodaPopper
                | WeaponClass::SniperrifleDecap
                | WeaponClass::Raygun
                | WeaponClass::MechanicalArm
                | WeaponClass::SniperrifleClassic
        )
    }

    pub fn is_projectile(&self) -> bool {
        matches!(
            self.weapon_class(),
            WeaponClass::RocketlauncherDirecthit
                | WeaponClass::Rocketlauncher
                | WeaponClass::CompoundBow
        )
    }

    pub fn uses_gravity(&self) -> bool {
        matches!(self.weapon_class(), WeaponClass::CompoundBow)
    }

    pub fn projectile_fire_offset(&self) -> Vec3 {
        match self.weapon_class() {
            WeaponClass::RocketlauncherDirecthit
            | WeaponClass::Rocketlauncher
            | WeaponClass::CompoundBow => Vec3::new(23.5, 12.0, -3.0),
            _ => Vec3::zero(),
        }
    }

    pub fn projectile_speed(&self) -> Option<f32> {
        match self.weapon_class() {
            WeaponClass::RocketlauncherDirecthit => Some(1980.0),
            WeaponClass::Rocketlauncher => Some(1100.0),
            WeaponClass::CompoundBow => Some(self.projectile_speed_()),
            _ => None,
        }
    }

    pub fn projectile_gravity(&self) -> Option<f32> {
        match self.weapon_class() {
            WeaponClass::CompoundBow => Some(self.projectile_gravity_()),
            _ => None,
        }
    }

    fn projectile_speed_(&self) -> f32 {
        let f = vfunc!(self.vtable, 542, extern "C" fn(*mut c_void) -> f32);
        f(self.this)
    }

    fn projectile_gravity_(&self) -> f32 {
        let f = vfunc!(self.vtable, 543, extern "C" fn(*mut c_void) -> f32);
        f(self.this)
    }
}

#[allow(dead_code)]
#[repr(i32)]
pub enum WeaponClass {
    None = 0,
    Bat,
    BatWood,
    Bottle,
    Fireaxe,
    Club,
    Crowbar,
    Knife,
    Fists,
    Shovel,
    Wrench,
    Bonesaw,
    ShotgunPrimary,
    ShotgunSoldier,
    ShotgunHwg,
    ShotgunPyro,
    Scattergun,
    Sniperrifle,
    Minigun,
    Smg,
    SyringegunMedic,
    Tranq,
    Rocketlauncher,
    Grenadelauncher,
    Pipebomblauncher,
    Flamethrower,
    GrenadeNormal,
    GrenadeConcussion,
    GrenadeNail,
    GrenadeMirv,
    GrenadeMirvDemoman,
    GrenadeNapalm,
    GrenadeGas,
    GrenadeEmp,
    GrenadeCaltrop,
    GrenadePipebomb,
    GrenadeSmokeBomb,
    GrenadeHeal,
    GrenadeStunball,
    GrenadeJar,
    GrenadeJarMilk,
    Pistol,
    PistolScout,
    Revolver,
    Nailgun,
    Pda,
    PdaEngineerBuild,
    PdaEngineerDestroy,
    PdaSpy,
    Builder,
    Medigun,
    GrenadeMirvbomb,
    FlamethrowerRocket,
    GrenadeDemoman,
    SentryBullet,
    SentryRocket,
    Dispenser,
    Invis,
    Flaregun,
    Lunchbox,
    Jar,
    CompoundBow,
    BuffItem,
    PumpkinBomb,
    Sword,
    RocketlauncherDirecthit,
    Lifeline,
    LaserPointer,
    DispenserGun,
    SentryRevenge,
    JarMilk,
    HandgunScoutPrimary,
    BatFish,
    Crossbow,
    Stickbomb,
    HandgunScoutSecondary,
    SodaPopper,
    SniperrifleDecap,
    Raygun,
    ParticleCannon,
    MechanicalArm,
    DrgPomson,
    BatGiftwrap,
    GrenadeOrnamentBall,
    FlaregunRevenge,
    PepBrawlerBlaster,
    Cleaver,
    GrenadeCleaver,
    StickyBallLauncher,
    GrenadeStickyBall,
    ShotgunBuildingRescue,
    Cannon,
    Throwable,
    GrenadeThrowable,
    PdaSpyBuild,
    GrenadeWaterballoon,
    HarvesterSaw,
    Spellbook,
    SpellbookProjectile,
    SniperrifleClassic,
    Parachute,
    Grapplinghook,
    PasstimeGun,
    ChargedSmg,
    Count,
}
