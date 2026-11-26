use std::{
    ffi::{CStr, c_void},
    ptr,
    sync::OnceLock,
};

use crate::{
    helpers::module,
    types::{Entity, Player},
};

/// Collision groups the game's rules decide collisions between
pub mod group {
    pub const NONE: i32 = 0;
    /// TF_COLLISIONGROUP_GRENADES, pipes and stickies
    pub const GRENADES: i32 = 20;
    /// TFCOLLISION_GROUP_ROCKETS
    pub const ROCKETS: i32 = 24;
}

/// What a filter lets through, past the entities it skips
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Everything
    All,
    /// Whatever the game's filter in `game` lets through
    Game,
}

/// A CTraceFilterSimple, or a filter built on one, laid out like the game's
#[repr(C)]
struct GameFilter {
    vtable: *const extern "C" fn(*const GameFilter, *const c_void, i32) -> bool,
    pass: *const c_void,
    group: i32,
    extra_check: *const c_void,
    /// CTraceFilterIgnoreTeammates' team
    ignore_team: i32,
}

#[repr(C)]
pub struct TraceFilter {
    vtable: *const *const c_void,
    skip: *const c_void,
    trace_type: i32,
    kind: Kind,
    /// A second entity to pass through
    skip_also: *const c_void,
    game: GameFilter,
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
            game: GameFilter {
                vtable: ptr::null(),
                pass: ptr::null(),
                group: group::NONE,
                extra_check: ptr::null(),
                ignore_team: 0,
            },
        }
    }

    /// The game's CTraceFilterSimple for something in `group` that `pass` owns: it passes
    /// through `pass` and what it owns, and collides with what the game's collision rules say.
    /// `skip_also` is passed through as well.
    pub fn simple(pass: &Player, group: i32, skip_also: Option<&Entity>) -> Self {
        Self::game(c"18CTraceFilterSimple", pass, group, 0, skip_also)
    }

    /// The game's CTraceFilterIgnoreTeammates, what aiming a projectile at the crosshair traces
    /// with: like CTraceFilterSimple, but through `pass`'s teammates
    pub fn ignore_teammates(pass: &Player) -> Self {
        let team = pass.team() as i32;
        Self::game(
            c"27CTraceFilterIgnoreTeammates",
            pass,
            group::NONE,
            team,
            None,
        )
    }

    fn game(
        name: &'static CStr,
        pass: &Player,
        group: i32,
        ignore_team: i32,
        skip_also: Option<&Entity>,
    ) -> Self {
        let mut filter = TraceFilter {
            skip_also: skip_also.map_or(ptr::null(), |p| p.this as _),
            ..TraceFilter::init(Some(pass))
        };
        if let Some(vtable) = game_vtable(name) {
            filter.kind = Kind::Game;
            filter.game = GameFilter {
                vtable: vtable as _,
                pass: pass.this as _,
                group,
                extra_check: ptr::null(),
                ignore_team,
            };
        }
        filter
    }

    /// Only the world and static props, no entities
    pub fn world_only() -> Self {
        TraceFilter {
            trace_type: 1, // TRACE_WORLD_ONLY
            ..TraceFilter::init(None)
        }
    }

    extern "C" fn should_hit_entity(filter: *const Self, entity: *const c_void, mask: i32) -> bool {
        let filter = unsafe { &*filter };
        if entity.is_null() || entity == filter.skip || entity == filter.skip_also {
            return false;
        }
        match filter.kind {
            Kind::All => true,
            Kind::Game => unsafe { (*filter.game.vtable)(&filter.game, entity, mask) },
        }
    }

    extern "C" fn get_trace_type(filter: *const Self) -> i32 {
        unsafe { (*filter).trace_type }
    }
}

/// The vtable of the game's trace filter class with the mangled RTTI name `name`
fn game_vtable(name: &'static CStr) -> Option<usize> {
    static SIMPLE: OnceLock<Option<usize>> = OnceLock::new();
    static IGNORE_TEAMMATES: OnceLock<Option<usize>> = OnceLock::new();
    let cell = if name == c"18CTraceFilterSimple" {
        &SIMPLE
    } else {
        &IGNORE_TEAMMATES
    };
    *cell.get_or_init(|| {
        let found = module::vtable_of(&module::segments("client.so"), name);
        if found.is_none() {
            log::warn!("{name:?} vtable not found, traces with it hit everything");
        }
        found
    })
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
