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

#[derive(PartialEq, Clone)]
pub enum EntityType {
    Creature,
    Projectile,
}

/// Server side representation of an entity to check movement.
#[derive(Clone)]
pub struct Entity {
    pub uuid: Uuid,
    pub entity_type: EntityType,
    obj: Object,
    pub faced_left: bool,
    pub last_trajectory: (f32, f32),
    pub has_moved: bool,
}

impl Entity {
    /// Creates a new entity including the rectangle for collision checkings.
    pub fn new(
        uuid: Uuid,
        position: (i32, i32, i8),
        size: (u16, u16),
        entity_type: EntityType,
    ) -> Self {
        let (x, y, z) = position;
        Self {
            uuid,
            entity_type,
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
        trajectory: (f32, f32),
        speed: f32,
    ) -> MoveQuery {
        let (tx, ty) = trajectory;
        let (bx, by) = boundary;
        let (x, y) = self.obj.top_left();
        let (w, h) = self.obj.size();

        // Apply movement deltas within bounds.
        let mut touch_boundary: bool = false;
        let (touched, dx) = Self::calc_coord_change(x, tx, w, bx, speed);
        touch_boundary |= touched;
        let (touched, dy) = Self::calc_coord_change(y, ty, h, by, speed);
        touch_boundary |= touched;

        // Builds the query.
        let mut query = MoveQuery {
            uuid: self.uuid,
            source: self.obj.position(),
            destination: if touch_boundary {
                self.obj.position()
            } else {
                (dx, dy, self.obj.z())
            },
            trajectory: if touch_boundary { (0.0, 0.0) } else { (tx, ty) },
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

    /// Calculates a coordinate change, if the boundary is touched or exceed, it notifies the caller.
    fn calc_coord_change(
        source: i32,
        trajectory: f32,
        size: u16,
        boundary: u32,
        speed: f32,
    ) -> (bool, i32) {
        // Apply movement deltas within bounds.
        let raw = source as f32 + (trajectory * speed);
        let lower = (boundary - size as u32) as f32;
        if raw < 0.0 {
            // Cannot be lower than 0 coordinate (top-left)
            (true, 0)
        } else if lower < raw {
            // Size exceeded a bottom-right coordinate.
            (true, lower.floor() as i32)
        } else {
            // No change required.
            (false, raw.floor() as i32)
        }
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
