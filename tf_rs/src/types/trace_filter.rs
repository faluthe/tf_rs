use std::ffi::c_void;

use crate::types::Player;

#[repr(C)]
pub struct TraceFilter {
    vtable: *mut *mut c_void,
    skip: *mut Player,
}

impl TraceFilter {
    pub fn init(ignore: Option<&Player>) -> Self {
        TraceFilter {
            vtable: &TRACE_FILTER_VTABLE as *const _ as *mut *mut c_void,
            skip: match ignore {
                Some(player) => player as *const Player as *mut Player,
                None => std::ptr::null_mut(),
            },
        }
    }

    extern "C" fn should_hit_entity(filter: *const Self, entity: *mut Player, _mask: i32) -> bool {
        entity != unsafe { (*filter).skip }
    }

    extern "C" fn get_trace_type(_filter: *const Self) -> i32 {
        0 // TRACE_EVERYTHING
    }
}

#[repr(C)]
struct TraceFilterVTable {
    should_hit_entity: extern "C" fn(*const TraceFilter, *mut Player, i32) -> bool,
    get_trace_type: extern "C" fn(*const TraceFilter) -> i32,
}

static TRACE_FILTER_VTABLE: TraceFilterVTable = TraceFilterVTable {
    should_hit_entity: TraceFilter::should_hit_entity,
    get_trace_type: TraceFilter::get_trace_type,
};
