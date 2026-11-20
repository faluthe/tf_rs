#[repr(C)]
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
    pub fov: f32,
}
