use std::ffi::{CStr, c_char, c_void};

use once_cell::sync::OnceCell;

use crate::{helpers::module, offset_get, traits::FromRaw, vfunc};

type AttribHookFloat = extern "C" fn(f32, *const c_char, *mut c_void, *mut c_void, bool) -> f32;

/// CAttributeManager::AttribHookValue<float>, called right after the melee swing trace loads
/// the name of the bounds multiplier
fn attrib_hook_float() -> Option<AttribHookFloat> {
    static HOOK: OnceCell<Option<usize>> = OnceCell::new();
    let addr = HOOK.get_or_init(|| {
        let segments = module::segments("client.so");
        let hook = module::find_string(&segments, c"melee_bounds_multiplier")
            .and_then(|name| module::find_lea_to(&segments, name))
            .and_then(|lea| module::call_target(lea + 7));
        if hook.is_none() {
            log::warn!("Attribute hook not found, weapon attributes are ignored");
        }
        hook
    });
    addr.map(|a| unsafe { std::mem::transmute::<usize, AttribHookFloat>(a) })
}

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
    offset_get!(fn weapon_mode: i32, 0xF70);
    // CTFWeaponBaseMelee: when the current swing lands, negative when not swinging
    offset_get!(pub fn smack_time: f32, 0x10A0);
    // CTFPipebombLauncher, and CTFCompoundBow which derives from it
    offset_get!(pub fn charge_begin_time: f32, 0x10D8);

    pub fn weapon_class(&self) -> WeaponClass {
        let f = vfunc!(self.vtable, 451, extern "C" fn(*mut c_void) -> WeaponClass);
        f(self.this)
    }

    /// Seconds from starting a melee swing until it lands
    pub fn smack_delay(&self) -> f32 {
        // Returns curtime plus the weapon mode's smack delay
        let f = vfunc!(self.vtable, 532, extern "C" fn(*mut c_void, i32) -> f32);
        (f(self.this, self.weapon_mode()) - crate::interfaces::Interfaces::global_vars().curtime)
            .max(0.0)
    }

    pub fn is_knife(&self) -> bool {
        matches!(self.weapon_class(), WeaponClass::Knife)
    }

    /// Reach of a melee swing before attributes and model scale
    pub fn swing_range(&self) -> f32 {
        let f = vfunc!(self.vtable, 528, extern "C" fn(*mut c_void) -> i32);
        f(self.this) as f32
    }

    /// `value` with the weapon's attribute `name` applied, unchanged if attributes can't be read
    pub fn attrib_float(&self, value: f32, name: &CStr) -> f32 {
        match attrib_hook_float() {
            Some(f) => f(value, name.as_ptr(), self.this, std::ptr::null_mut(), true),
            None => value,
        }
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

    pub fn is_melee(&self) -> bool {
        matches!(
            self.weapon_class(),
            WeaponClass::Bat
                | WeaponClass::BatWood
                | WeaponClass::Bottle
                | WeaponClass::Fireaxe
                | WeaponClass::Club
                | WeaponClass::Crowbar
                | WeaponClass::Knife
                | WeaponClass::Fists
                | WeaponClass::Shovel
                | WeaponClass::Wrench
                | WeaponClass::Bonesaw
                | WeaponClass::Sword
                | WeaponClass::BatFish
                | WeaponClass::BatGiftwrap
                | WeaponClass::Stickbomb
                | WeaponClass::HarvesterSaw
        )
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
