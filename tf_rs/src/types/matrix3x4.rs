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
}
