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
}

impl Vec3 {
    pub fn dot(&self, other: &Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn length(&self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn length_2d(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn distance_to(&self, other: &Vec3) -> f32 {
        (*self - *other).length()
    }

    pub fn to_2d(self) -> Vec3 {
        Vec3::new(self.x, self.y, 0.0)
    }

    /// Unit length, or zero for a zero vector
    pub fn normalized(self) -> Vec3 {
        let len = self.length();
        if len > f32::EPSILON {
            self * (1.0 / len)
        } else {
            Vec3::default()
        }
    }

    pub fn is_zero(&self) -> bool {
        self.x == 0.0 && self.y == 0.0 && self.z == 0.0
    }

    /// Yaw of the vector in degrees
    pub fn yaw(&self) -> f32 {
        self.y.atan2(self.x).to_degrees()
    }

    /// Forward, right and up vectors of view angles
    pub fn angle_vectors(&self) -> (Vec3, Vec3, Vec3) {
        let (sp, cp) = self.x.to_radians().sin_cos();
        let (sy, cy) = self.y.to_radians().sin_cos();
        let (sr, cr) = self.z.to_radians().sin_cos();
        let forward = Vec3::new(cp * cy, cp * sy, -sp);
        let right = Vec3::new(-sr * sp * cy + cr * sy, -sr * sp * sy - cr * cy, -sr * cp);
        let up = Vec3::new(cr * sp * cy + sr * sy, cr * sp * sy - sr * cy, cr * cp);
        (forward, right, up)
    }

    /// View angles pointing along the vector
    pub fn to_angles(&self) -> Vec3 {
        let pitch = (-self.z).atan2(self.length_2d()).to_degrees();
        Vec3::new(pitch, self.yaw(), 0.0)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: f32) -> Vec3 {
        Vec3::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Vec3) {
        *self = *self + rhs;
    }
}

/// Angle in degrees wrapped to -180..180
pub fn normalize_angle(a: f32) -> f32 {
    let a = (a + 180.0).rem_euclid(360.0) - 180.0;
    if a == -180.0 { 180.0 } else { a }
}
