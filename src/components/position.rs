use crate::impl_component;

use super::{Bounds, Vec2, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub size: Vec2,
    pub loc: Vec3,
}

impl Position {
    pub fn new(position: Vec3, size: Vec2) -> Self {
        Self {
            loc: position,
            size,
        }
    }

    pub fn bounds(&self) -> Bounds {
        Bounds::from_vec(self.loc, self.size)
    }
}

impl_component!(Position);
