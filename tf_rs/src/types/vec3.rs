#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn _new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }
}
