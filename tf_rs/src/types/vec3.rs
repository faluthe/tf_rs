#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub fn fov_to(&self, other: &Vec3) -> f32 {
        let x = (self.x - other.x).rem_euclid(360.0);
        let y = (self.y - other.y).rem_euclid(360.0);

        let clamped_x = x.clamp(-89.0, 89.0);
        let clamped_y = y.clamp(-180.0, 180.0);

        clamped_x.hypot(clamped_y)
    }
}
