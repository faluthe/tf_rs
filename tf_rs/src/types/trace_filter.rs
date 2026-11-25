use std::{ffi::c_void, ptr};

use crate::{
    traits::FromRaw,
    types::{Entity, Player},
};

/// m_iTeamNum
const TEAM: usize = 0xDC;

/// What a filter lets through, past the entity it skips
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Everything
    All,
    /// What a projectile fired by `team` runs into
    Projectile,
}

#[repr(C)]
pub struct TraceFilter {
    vtable: *const *const c_void,
    skip: *const c_void,
    trace_type: i32,
    kind: Kind,
    /// A second entity to pass through
    skip_also: *const c_void,
    team: i32,
}

impl TraceFilter {
    pub fn init(ignore: Option<&Player>) -> Self {
        TraceFilter {
            vtable: &TRACE_FILTER_VTABLE as *const _ as _,
            skip: match ignore {
                Some(player) => player.this as _,
                None => ptr::null(),
            },
            trace_type: 0, // TRACE_EVERYTHING
            kind: Kind::All,
            skip_also: ptr::null(),
            team: 0,
        }
    }

    /// What a projectile `shooter` fires collides with: the world, props, doors and other solid
    /// map entities, and enemy players, buildings and shields. It passes through teammates, and
    /// through `target`, whose box is checked separately where it's predicted to be.
    pub fn projectile(shooter: &Player, target: Option<&Player>) -> Self {
        TraceFilter {
            kind: Kind::Projectile,
            skip_also: target.map_or(ptr::null(), |t| t.this as _),
            team: unsafe { *shooter.field::<i32>(TEAM) },
            ..TraceFilter::init(Some(shooter))
        }
    }

    /// Only the world and static props, no entities
    pub fn world_only() -> Self {
        TraceFilter {
            trace_type: 1, // TRACE_WORLD_ONLY
            ..TraceFilter::init(None)
        }
    }

    extern "C" fn should_hit_entity(
        filter: *const Self,
        entity: *const c_void,
        _mask: i32,
    ) -> bool {
        let filter = unsafe { &*filter };
        if entity.is_null() || entity == filter.skip || entity == filter.skip_also {
            return false;
        }
        match filter.kind {
            Kind::All => true,
            Kind::Projectile => projectile_hits(filter.team, entity),
        }
    }

    extern "C" fn get_trace_type(filter: *const Self) -> i32 {
        unsafe { (*filter).trace_type }
    }
}

#[repr(C)]
struct TraceFilterVTable {
    should_hit_entity: extern "C" fn(*const TraceFilter, *const c_void, i32) -> bool,
    get_trace_type: extern "C" fn(*const TraceFilter) -> i32,
}

static TRACE_FILTER_VTABLE: TraceFilterVTable = TraceFilterVTable {
    should_hit_entity: TraceFilter::should_hit_entity,
    get_trace_type: TraceFilter::get_trace_type,
};

/// Whether a projectile from `team` collides with `entity`.
/// Static props and the world aren't passed to the filter, they're always hit.
fn projectile_hits(team: i32, entity: *const c_void) -> bool {
    let entity = Entity::from_raw(entity as *mut c_void);
    let enemy = || unsafe { *entity.field::<i32>(TEAM) } != team;
    match entity.class_name() {
        "CWorld"
        | "CBaseEntity"
        | "CBaseDoor"
        | "CDynamicProp"
        | "CPhysicsProp"
        | "CPhysicsPropMultiplayer"
        | "CFunc_LOD"
        | "CObjectCartDispenser"
        | "CFuncTrackTrain"
        | "CFuncConveyor"
        | "CTFGenericBomb"
        | "CTFPumpkinBomb"
        | "CObjectTeleporter" => true,
        "CTFPlayer" | "CTFMedigunShield" | "CBaseObject" | "CObjectSentrygun"
        | "CObjectDispenser" => enemy(),
        _ => false,
    }
}
