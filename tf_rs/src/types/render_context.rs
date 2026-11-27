use std::ffi::c_void;

use crate::{traits::FromRaw, types::Material, vfunc};

/// What the stencil buffer does with a pixel, StencilOperation_t
#[allow(dead_code)]
#[repr(i32)]
#[derive(Clone, Copy)]
pub enum StencilOp {
    Keep = 1,
    Zero = 2,
    Replace = 3,
    IncrSat = 4,
}

/// How a pixel's stencil value is tested against the reference, StencilComparisonFunction_t
#[allow(dead_code)]
#[repr(i32)]
#[derive(Clone, Copy)]
pub enum StencilFunc {
    Never = 1,
    Equal = 3,
    NotEqual = 6,
    Always = 8,
}

/// Every part of the stencil state, set at once
#[derive(Clone, Copy)]
pub struct StencilState {
    pub enable: bool,
    pub fail: StencilOp,
    pub z_fail: StencilOp,
    pub pass: StencilOp,
    pub func: StencilFunc,
    pub reference: i32,
    pub test_mask: u32,
    pub write_mask: u32,
}

impl StencilState {
    /// Stencil testing off, everything else at its default
    pub const DISABLED: StencilState = StencilState {
        enable: false,
        fail: StencilOp::Keep,
        z_fail: StencilOp::Keep,
        pass: StencilOp::Keep,
        func: StencilFunc::Always,
        reference: 0,
        test_mask: 0xFFFF_FFFF,
        write_mask: 0xFFFF_FFFF,
    };
}

/// A texture, here only the render targets
#[derive(Copy, Clone)]
pub struct Texture {
    pub this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl FromRaw for Texture {
    fn from_raw(raw: *mut c_void) -> Self {
        let vtable = unsafe { *(raw as *mut *mut *mut c_void) };
        Texture { this: raw, vtable }
    }
}

impl Texture {
    pub fn actual_width(&self) -> i32 {
        let f = vfunc!(self.vtable, 3, (*mut c_void) -> i32);
        f(self.this)
    }

    pub fn actual_height(&self) -> i32 {
        let f = vfunc!(self.vtable, 4, (*mut c_void) -> i32);
        f(self.this)
    }
}

/// The material system's render context, held between BeginRender and EndRender like the
/// game's CMatRenderContextPtr, and released when dropped
pub struct RenderContext {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl RenderContext {
    /// Take `raw` from IMaterialSystem::GetRenderContext, which already holds a reference
    pub fn begin(raw: *mut c_void) -> Self {
        let vtable = unsafe { *(raw as *mut *mut *mut c_void) };
        let ctx = RenderContext { this: raw, vtable };
        let f = vfunc!(ctx.vtable, 2, (*mut c_void) -> ());
        f(ctx.this);
        ctx
    }

    pub fn set_render_target(&self, texture: Option<&Texture>) {
        let f = vfunc!(self.vtable, 6, (*mut c_void, *mut c_void) -> ());
        f(self.this, texture.map_or(std::ptr::null_mut(), |t| t.this));
    }

    pub fn clear_buffers(&self, color: bool, depth: bool, stencil: bool) {
        let f = vfunc!(self.vtable, 12, (*mut c_void, bool, bool, bool) -> ());
        f(self.this, color, depth, stencil);
    }

    pub fn viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        let f = vfunc!(self.vtable, 38, (*mut c_void, i32, i32, i32, i32) -> ());
        f(self.this, x, y, width, height);
    }

    /// x, y, width and height
    pub fn get_viewport(&self) -> [i32; 4] {
        let f = vfunc!(
            self.vtable,
            39,
            (*mut c_void, *mut i32, *mut i32, *mut i32, *mut i32) -> ()
        );
        let mut v = [0; 4];
        let [x, y, w, h] = &mut v;
        f(self.this, x, y, w, h);
        v
    }

    pub fn clear_color3ub(&self, r: u8, g: u8, b: u8) {
        let f = vfunc!(self.vtable, 72, (*mut c_void, u8, u8, u8) -> ());
        f(self.this, r, g, b);
    }

    /// With `enable`, depth writes are forced on or off by `depth_write`
    pub fn override_depth_enable(&self, enable: bool, depth_write: bool) {
        let f = vfunc!(self.vtable, 74, (*mut c_void, bool, bool) -> ());
        f(self.this, enable, depth_write);
    }

    /// Draw `material` over the destination rectangle, sampling the source rectangle of a
    /// `src_size` texture
    pub fn draw_screen_space_rectangle(
        &self,
        material: &Material,
        dest: [i32; 4],
        src: [f32; 4],
        src_size: [i32; 2],
    ) {
        let f = vfunc!(
            self.vtable,
            103,
            (
                *mut c_void,
                *mut c_void,
                i32,
                i32,
                i32,
                i32,
                f32,
                f32,
                f32,
                f32,
                i32,
                i32,
                *mut c_void,
                i32,
                i32
            ) -> ()
        );
        let [x, y, w, h] = dest;
        let [x0, y0, x1, y1] = src;
        let [tw, th] = src_size;
        f(
            self.this,
            material.this,
            x,
            y,
            w,
            h,
            x0,
            y0,
            x1,
            y1,
            tw,
            th,
            std::ptr::null_mut(),
            1,
            1,
        );
    }

    pub fn push_render_target_and_viewport(&self) {
        let f = vfunc!(self.vtable, 105, (*mut c_void) -> ());
        f(self.this);
    }

    pub fn pop_render_target_and_viewport(&self) {
        let f = vfunc!(self.vtable, 109, (*mut c_void) -> ());
        f(self.this);
    }

    pub fn set_stencil_state(&self, state: &StencilState) {
        let set_bool =
            |i: usize, v: bool| vfunc!(self.vtable, i, (*mut c_void, bool) -> ())(self.this, v);
        let set_int =
            |i: usize, v: i32| vfunc!(self.vtable, i, (*mut c_void, i32) -> ())(self.this, v);
        set_bool(117, state.enable);
        set_int(118, state.fail as i32);
        set_int(119, state.z_fail as i32);
        set_int(120, state.pass as i32);
        set_int(121, state.func as i32);
        set_int(122, state.reference);
        set_int(123, state.test_mask as i32);
        set_int(124, state.write_mask as i32);
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        let end_render = vfunc!(self.vtable, 3, (*mut c_void) -> ());
        end_render(self.this);
        let release = vfunc!(self.vtable, 1, (*mut c_void) -> i32);
        release(self.this);
    }
}
