#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub fn zero() -> Self {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn fov_to(&self, other: &Vec3) -> f32 {
        let mut dx = (self.x - other.x).rem_euclid(360.0);
        let mut dy = (self.y - other.y).rem_euclid(360.0);

        if dx > 180.0 {
            dx -= 360.0;
        }
        if dy > 180.0 {
            dy -= 360.0;
        }

        dx = dx.clamp(-89.0, 89.0);
        dy = dy.clamp(-180.0, 180.0);

        (dx * dx) + (dy * dy).sqrt()
    }

    pub fn distance_to(&self, other: &Vec3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}
