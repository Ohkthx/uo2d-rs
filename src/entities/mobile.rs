use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use uuid::Uuid;

use crate::components::{Bounds, Transform, Vec2, Vec3};
use crate::spatial_hash::SpatialHash;

use super::Region;

/// A query to move an entity. Useful to check multiple movements in 1 tick.
#[derive(Debug)]
pub struct MoveQuery {
    pub uuid: Uuid,
    pub source: Vec3,
    pub destination: Vec3,
    pub velocity: Vec2,
    pub entity_size: (f64, f64),
    pub nearby: HashSet<Uuid>,
}

impl MoveQuery {
    /// Checks if a move has happened.
    pub fn has_moved(&self) -> bool {
        self.source.round() != self.destination.round()
    }

    /// Checks if the entitiy is potentialy stuck against a boundary or collision.
    pub fn is_stuck(&self) -> bool {
        !self.has_moved() && self.velocity != Vec2::ORIGIN
    }
}

#[derive(PartialEq, Clone)]
pub enum MobileType {
    Creature,
    Projectile,
}

/// Server side representation of an entity to check movement.
#[derive(Clone)]
pub struct Mobile {
    pub uuid: Uuid,
    pub mobile_type: MobileType,
    pub transform: Transform,
    pub last_position: Vec3,
    pub faced_left: bool,
    pub last_velocity: Vec2,
    pub has_moved: bool,
}

impl Mobile {
    /// Creates a new entity including the rectangle for collision checkings.
    pub fn new(uuid: Uuid, coord: Vec3, size: Vec2, mobile_type: MobileType) -> Self {
        Self {
            uuid,
            mobile_type,
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

    /// Applies the movement to the entity, modifying the position and size if necessary.
    fn apply_move(&mut self, position: Vec3) {
        self.last_position = self.position();
        self.transform.set_position(&position);
    }

    /// Checks the entities attempted movement to ensure it is within the boundaries. Returns a MoveQuery used to check collision with other entities.
    pub fn check_move(
        &self,
        spatial_area: &mut SpatialHash,
        region: &Region,
        velocity: Vec2,
    ) -> MoveQuery {
        // Apply movement deltas within bounds.
        let transform = self
            .transform
            .applied_velocity(&velocity, &region.bounding_box());
        let mut bounds = self.bounding_box();

        // Builds the query.
        let mut query = MoveQuery {
            uuid: self.uuid,
            source: self.transform.position(),
            destination: transform.position(),
            velocity,
            entity_size: self.bounding_box().dimensions().as_tuple(),
            nearby: HashSet::new(),
        };

        // Get nearby entities.
        if query.has_moved() {
            bounds.set_x(transform.position().x());
            bounds.set_y(transform.position().y());
            query.nearby = spatial_area.query(&bounds, Some(&self.uuid));
        }

        query
    }

    /// Finalizes the movement utilizing the query. Updates the spatial hash with the new position.
    pub fn move_entity(&mut self, spatial_area: &mut SpatialHash, query: &MoveQuery) {
        if !query.is_stuck() {
            self.has_moved = query.has_moved();
            self.last_velocity = query.velocity;
            spatial_area.remove_object(&self.uuid, &self.bounding_box());
            self.apply_move(query.destination);
            self.faced_left = query.destination.x() < self.transform.position().x();
            spatial_area.insert_object(&self.uuid, &self.bounding_box());
        } else {
            self.last_velocity = Vec2::ORIGIN;
            self.has_moved = query.has_moved();
        }
    }
}

impl PartialEq for Mobile {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Eq for Mobile {}

impl Hash for Mobile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}
