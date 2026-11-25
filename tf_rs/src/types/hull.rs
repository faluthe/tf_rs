use crate::types::Vec3;

/// A collision box at a position
#[derive(Clone, Copy)]
pub struct Hull {
    pub origin: Vec3,
    pub mins: Vec3,
    pub maxs: Vec3,
}
