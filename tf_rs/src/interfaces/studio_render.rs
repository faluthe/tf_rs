use std::{ffi::c_void, ptr};

use crate::{
    types::{Matrix3x4, Vec3},
    vfunc,
};

/// Skip flex animation, nothing drives it
pub const STUDIORENDER_DRAW_NO_FLEXES: i32 = 0x04;

/// A light other than the ambient ones, unused but part of DrawModelInfo_t
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LightDesc {
    pub kind: i32,
    pub color: Vec3,
    pub position: Vec3,
    pub direction: Vec3,
    pub range: f32,
    pub falloff: f32,
    pub attenuation: [f32; 3],
    pub theta: f32,
    pub phi: f32,
    pub theta_dot: f32,
    pub phi_dot: f32,
    pub flags: u32,
    pub one_over_theta_dot_minus_phi_dot: f32,
    pub range_squared: f32,
}

#[repr(C)]
pub struct DrawModelInfo {
    pub studio_hdr: *const u8,
    pub hardware_data: *mut c_void,
    pub decals: *mut c_void,
    pub skin: i32,
    pub body: i32,
    pub hitbox_set: i32,
    pub client_entity: *mut c_void,
    pub lod: i32,
    pub color_meshes: *mut c_void,
    pub static_lighting: bool,
    pub ambient_cube: [Vec3; 6],
    pub local_light_count: i32,
    pub local_lights: [LightDesc; 4],
}

/// Draws models straight from their files, with no entity or map behind them
#[derive(Default, Clone)]
pub struct StudioRender {
    this: *mut c_void,
    vtable: *mut *mut c_void,
}

impl StudioRender {
    pub fn new(this: *mut c_void) -> Self {
        let vtable = unsafe { *(this as *mut *mut *mut c_void) };
        StudioRender { this, vtable }
    }

    pub fn is_valid(&self) -> bool {
        !self.this.is_null()
    }

    /// Point the eyes of the model at `target`
    pub fn set_eye_view_target(&self, studio_hdr: *const u8, body: i32, target: &Vec3) {
        let f = vfunc!(self.vtable, 13, (*mut c_void, *const u8, i32, *const Vec3) -> ());
        f(self.this, studio_hdr, body, target);
    }

    /// Light from each of +x, -x, +y, -y, +z and -z
    pub fn set_ambient_light_colors(&self, colors: &[Vec3; 6]) {
        let f = vfunc!(self.vtable, 17, (*mut c_void, *const Vec3) -> ());
        f(self.this, colors.as_ptr());
    }

    pub fn clear_local_lights(&self) {
        let f = vfunc!(self.vtable, 18, (*mut c_void, i32, *const LightDesc) -> ());
        f(self.this, 0, ptr::null());
    }

    pub fn set_view_state(&self, origin: &Vec3, right: &Vec3, up: &Vec3, forward: &Vec3) {
        let f = vfunc!(
            self.vtable,
            19,
            (*mut c_void, *const Vec3, *const Vec3, *const Vec3, *const Vec3) -> ()
        );
        f(self.this, origin, right, up, forward);
    }

    /// Storage for `count` bone matrices that lives as long as the draw needs it
    fn lock_bone_matrices(&self, count: i32) -> *mut Matrix3x4 {
        let f = vfunc!(self.vtable, 22, (*mut c_void, i32) -> *mut Matrix3x4);
        f(self.this, count)
    }

    fn unlock_bone_matrices(&self) {
        let f = vfunc!(self.vtable, 23, (*mut c_void) -> ());
        f(self.this);
    }

    pub fn set_color_modulation(&self, color: [f32; 3]) {
        let f = vfunc!(self.vtable, 27, (*mut c_void, *const f32) -> ());
        f(self.this, color.as_ptr());
    }

    pub fn set_alpha_modulation(&self, alpha: f32) {
        let f = vfunc!(self.vtable, 28, (*mut c_void, f32) -> ());
        f(self.this, alpha);
    }

    /// Draw the model in `info` posed by `bones`, one matrix per bone of the model
    pub fn draw_model(&self, info: &DrawModelInfo, bones: &[Matrix3x4], origin: &Vec3, flags: i32) {
        let locked = self.lock_bone_matrices(bones.len() as i32);
        if locked.is_null() {
            return;
        }
        unsafe { ptr::copy_nonoverlapping(bones.as_ptr(), locked, bones.len()) };
        self.unlock_bone_matrices();

        let f = vfunc!(
            self.vtable,
            29,
            (
                *mut c_void,
                *mut c_void,
                *const DrawModelInfo,
                *mut Matrix3x4,
                *mut f32,
                *mut f32,
                *const Vec3,
                i32
            ) -> ()
        );
        f(
            self.this,
            ptr::null_mut(),
            info,
            locked,
            ptr::null_mut(),
            ptr::null_mut(),
            origin,
            flags,
        );
    }
}
