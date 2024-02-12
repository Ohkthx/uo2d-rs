use std::collections::HashSet;

use uuid::Uuid;

use crate::object::{Object, Position};
use crate::spatial_hash::SpatialHash;

/// A query to move an entity. Useful to check multiple movements in 1 tick.
pub struct MoveQuery {
    pub uuid: Uuid,
    pub source: Position,
    pub destination: Position,
    pub trajectory: (f32, f32),
    pub entity_size: (u16, u16),
    pub nearby: HashSet<Uuid>,
}

impl MoveQuery {
    /// Checks if a move has happened.
    pub fn has_moved(&self) -> bool {
        self.source != self.destination
    }

    /// Checks if the entitiy is potentialy stuck against a boundary or collision.
    pub fn is_stuck(&self) -> bool {
        !self.has_moved() && self.trajectory != (0.0, 0.0)
    }
}

/// Server side representation of an entity to check movement.
pub struct Entity {
    pub uuid: Uuid,
    obj: Object,
    pub faced_left: bool,
    pub last_trajectory: (f32, f32),
    pub has_moved: bool,
}

impl Entity {
    /// Creates a new entity including the rectangle for collision checkings.
    pub fn new(uuid: Uuid, position: (i32, i32, i8), size: (u16, u16)) -> Self {
        let (x, y, z) = position;
        Self {
            uuid,
            obj: Object::new(x, y, z, size.0, size.1),
            faced_left: false,
            last_trajectory: (0.0, 0.0),
            has_moved: false,
        }
    }

    /// Contains the boundaries and size of the entity.
    pub fn object(&self) -> &Object {
        &self.obj
    }

    /// Applies the movement to the entity, modifying the position and size if necessary.
    fn apply_move(&mut self, position: Position, width: u16, height: u16) {
        self.obj.update(position, width, height);
    }

    /// Checks the entities attempted movement to ensure it is within the boundaries. Returns a MoveQuery used to check collision with other entities.
    pub fn check_move(
        &self,
        spatial_area: &mut SpatialHash,
        boundary: (u32, u32),
        _source: Position,
        trajectory: (f32, f32),
        speed: f32,
    ) -> MoveQuery {
        let (tx, ty) = trajectory;

        // Apply movement deltas within bounds.
        let dx = (self.obj.x() as f32 + (tx * speed))
            .max(0.0)
            .min((boundary.0 - self.obj.width_u32()) as f32)
            .floor() as i32;
        let dy = (self.obj.y() as f32 + (ty * speed))
            .max(0.0)
            .min((boundary.1 - self.obj.height_u32()) as f32)
            .floor() as i32;

        // Builds the query.
        let mut query = MoveQuery {
            uuid: self.uuid,
            source: (self.obj.x(), self.obj.y(), self.obj.z()),
            destination: (dx, dy, self.obj.z()),
            trajectory: (tx, ty),
            entity_size: self.obj.size(),
            nearby: HashSet::new(),
        };

        // Get nearby entities.
        if query.has_moved() {
            let new = Object::new(dx, dy, self.obj.z(), self.obj.width(), self.obj.height());
            query.nearby = spatial_area.query(&new, Some(self.uuid));
        }

        query
    }

    /// Finalizes the movement utilizing the query. Updates the spatial hash with the new position.
    pub fn move_entity(&mut self, spatial_area: &mut SpatialHash, query: &MoveQuery) {
        if !query.is_stuck() {
            self.has_moved = query.has_moved();
            self.last_trajectory = query.trajectory;
            spatial_area.remove_entity(self);
            self.apply_move(query.destination, query.entity_size.0, query.entity_size.1);
            self.faced_left = query.destination.0 < self.obj.x();
            spatial_area.insert_entity(self);
        } else {
            self.last_trajectory = (0.0, 0.0);
            self.has_moved = query.has_moved();
        }
    }
}