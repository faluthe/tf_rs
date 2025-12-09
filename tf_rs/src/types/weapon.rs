use std::ffi::c_void;

use crate::{offset_get, traits::FromRaw, vfunc};

pub struct Weapon {
    this: *mut c_void,
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
