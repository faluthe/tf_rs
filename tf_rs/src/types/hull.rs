use crate::types::Vec3;

/// A collision box at a position
#[derive(Clone, Copy)]
pub struct Hull {
    pub origin: Vec3,
    pub mins: Vec3,
    pub maxs: Vec3,
}

impl Hull {
    /// Point of the box closest to `point`
    pub fn nearest_point(&self, point: &Vec3) -> Vec3 {
        let (o, lo, hi) = (&self.origin, &self.mins, &self.maxs);
        Vec3::new(
            point.x.clamp(o.x + lo.x, o.x + hi.x),
            point.y.clamp(o.y + lo.y, o.y + hi.y),
            point.z.clamp(o.z + lo.z, o.z + hi.z),
        )
    }

    /// Middle of the box
    pub fn center(&self) -> Vec3 {
        self.origin + Vec3::new(0.0, 0.0, (self.mins.z + self.maxs.z) / 2.0)
    }
}
