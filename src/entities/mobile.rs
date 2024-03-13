use std::hash::{Hash, Hasher};

use crate::components::{Bounds, Transform, Vec2, Vec3};
use crate::ecs::Entity;

/// Server side representation of an entity to check movement.
#[derive(Clone)]
pub struct Mobile {
    pub entity: Entity,
    pub transform: Transform,
    pub last_position: Vec3,
    pub faced_left: bool,
    pub last_velocity: Vec2,
    pub has_moved: bool,
}

impl Mobile {
    /// Creates a new entity including the rectangle for collision checkings.
    pub fn new(entity: Entity, coord: Vec3, size: Vec2) -> Self {
        Self {
            entity,
            transform: Transform::from_vecs(coord, size),
            last_position: Vec3::ORIGIN,
            faced_left: false,
            last_velocity: Vec2::ORIGIN,
            has_moved: false,
        }
    }

    /// Obtains the bounding box of the entity.
    pub fn bounding_box(&self) -> Bounds {
        self.transform.bounding_box()
    }

    /// Current position of the entity.
    pub fn position(&self) -> Vec3 {
        self.bounding_box().top_left_3d()
    }

    /// Maximum size of the collision box for the entity.
    pub fn size(&self) -> Vec2 {
        self.transform.bounding_box().dimensions()
    }
}

impl PartialEq for Mobile {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
    }
}

impl Eq for Mobile {}

impl Hash for Mobile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity.hash(state);
    }
}
