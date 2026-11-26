use std::{ffi::c_void, ptr};

use crate::{
    types::{Material, Vec3, entity::Renderable},
    vfunc,
};

/// DrawModel flag to draw the model
pub const STUDIO_RENDER: i32 = 0x1;

#[derive(Default, Clone)]
pub struct ModelRender {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl ModelRender {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        ModelRender { this, vtable }
    }

    pub fn ptr(&self) -> *mut c_void {
        self.this
    }

    pub fn forced_material_override(&self, material: Option<&Material>) {
        let f = vfunc!(self.vtable, 1, (*mut c_void, *mut c_void, i32) -> ());
        f(self.this, material.map_or(ptr::null_mut(), |m| m.this), 0);
    }

    /// Draw the model of `renderable`, entity `index`, the way the engine draws entities, so
    /// DrawModelExecute sees it. Draws regardless of whether the entity would draw itself
    pub fn draw_model(&self, renderable: &Renderable, index: i32, origin: &Vec3, angles: &Vec3) {
        let f = vfunc!(
            self.vtable,
            0,
            (
                *mut c_void,
                i32,
                *mut c_void,
                u16,
                i32,
                *const c_void,
                *const Vec3,
                *const Vec3,
                i32,
                i32,
                i32,
                *const c_void,
                *const c_void
            ) -> i32
        );
        f(
            self.this,
            STUDIO_RENDER,
            renderable.this,
            renderable.model_instance(),
            index,
            renderable.model(),
            origin,
            angles,
            renderable.skin(),
            renderable.body(),
            0,
            ptr::null(),
            ptr::null(),
        );
    }
}
