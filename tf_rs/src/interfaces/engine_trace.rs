use std::ffi::c_void;

use crate::{
    types::{Player, Ray, TraceFilter, TraceResult, Vec3},
    vfunc,
};

#[derive(Default, Clone)]
pub struct EngineTrace {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl EngineTrace {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        EngineTrace { this, vtable }
    }

    pub fn trace_ray(
        &self,
        from: &Vec3,
        to: &Vec3,
        mask: u32,
        ignore: Option<&Player>,
    ) -> TraceResult {
        let ray = Ray::init(from, to);
        let filter = TraceFilter::init(ignore);
        let mut result = TraceResult::default();

        let f = vfunc!(
            self.vtable,
            4,
            extern "C" fn(*mut c_void, *mut c_void, u32, *mut c_void, *mut TraceResult) -> ()
        );

        f(
            self.this,
            &ray as *const _ as *mut c_void,
            mask,
            &filter as *const _ as *mut c_void,
            &mut result,
        );
        result
    }
}
