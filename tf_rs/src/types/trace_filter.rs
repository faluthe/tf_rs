use std::{ffi::c_void, ptr};

use crate::types::Player;

#[repr(C)]
pub struct TraceFilter {
    vtable: *const *const c_void,
    skip: *const c_void,
}

impl TraceFilter {
    pub fn init(ignore: Option<&Player>) -> Self {
        TraceFilter {
            vtable: &TRACE_FILTER_VTABLE as *const _ as _,
            skip: match ignore {
                Some(player) => player.this as _,
                None => ptr::null(),
            },
        }
    }

    extern "C" fn should_hit_entity(
        filter: *const Self,
        entity: *const c_void,
        _mask: i32,
    ) -> bool {
        entity != unsafe { (*filter).skip }
    }

    extern "C" fn get_trace_type(_filter: *const Self) -> i32 {
        0 // TRACE_EVERYTHING
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
