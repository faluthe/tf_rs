use crate::types::Vec3;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Matrix3x4 {
    pub m: [[f32; 4]; 3],
}

impl Matrix3x4 {
    /// Translation part of the matrix
    pub fn position(&self) -> Vec3 {
        Vec3::new(self.m[0][3], self.m[1][3], self.m[2][3])
    }

    /// `point` moved from the matrix's space into the world
    pub fn transform(&self, point: &Vec3) -> Vec3 {
        let row = |r: &[f32; 4]| r[0] * point.x + r[1] * point.y + r[2] * point.z + r[3];
        Vec3::new(row(&self.m[0]), row(&self.m[1]), row(&self.m[2]))
    }
}
