use crate::types::Vec3;

#[repr(C)]
#[derive(Default)]
pub struct ViewSetup {
    pub x: i32,
    pub unscaled_x: i32,
    pub y: i32,
    pub unscaled_y: i32,
    pub width: i32,
    pub unscaled_width: i32,
    pub height: i32,
    pub stereo_eye: i32,
    pub unscaled_height: i32,
    pub ortho: bool,
    pub ortho_left: f32,
    pub ortho_top: f32,
    pub ortho_right: f32,
    pub ortho_bottom: f32,
    /// Horizontal
    pub fov: f32,
    pub fov_viewmodel: f32,
    pub origin: Vec3,
    pub angles: Vec3,
    pub z_near: f32,
    pub z_far: f32,
    pub z_near_viewmodel: f32,
    pub z_far_viewmodel: f32,
    pub render_to_subrect_of_larger_screen: bool,
    pub aspect_ratio: f32,
    pub off_center: bool,
    pub off_center_top: f32,
    pub off_center_bottom: f32,
    pub off_center_left: f32,
    pub off_center_right: f32,
    pub do_bloom_and_tone_mapping: bool,
    pub cache_full_scene_state: bool,
    pub view_to_projection_override: bool,
    pub view_to_projection: [[f32; 4]; 4],
}

/// The six planes, normal and distance each, the engine fills when a view is pushed
pub type Frustum = [[f32; 4]; 6];

pub const VIEW_CLEAR_DEPTH: i32 = 0x2;
