use std::collections::HashSet;

use sdl2::rect::Rect;
use uuid::Uuid;

use crate::spatial_hash::SpatialHash;

/// A query to move an entity. Useful to check multiple movements in 1 tick.
pub struct MoveQuery {
    pub uuid: Uuid,
    pub source: (i32, i32),
    pub destination: (i32, i32),
    pub trajectory: (f32, f32),
    pub entity_size: (u32, u32),
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
    rect: Rect,
    pub faced_left: bool,
    pub last_trajectory: (f32, f32),
    pub has_moved: bool,
}

impl Entity {
    /// Creates a new entity including the rectangle for collision checkings.
    pub fn new(uuid: Uuid, position: (i32, i32), size: u16) -> Self {
        Self {
            uuid,
            rect: Rect::new(position.0, position.1, size as u32, size as u32),
            faced_left: false,
            last_trajectory: (0.0, 0.0),
            has_moved: false,
        }
    }

    /// Contains the boundaries and size of the entity.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Applies the movement to the entity, modifying the position and size if necessary.
    fn apply_move(&mut self, position: (i32, i32), size: u16) {
        self.rect.set_x(position.0);
        self.rect.set_y(position.1);
        if self.rect.size() != (size as u32, size as u32) {
            self.rect.set_width(size as u32);
            self.rect.set_height(size as u32);
        }
    }

    /// Checks the entities attempted movement to ensure it is within the boundaries. Returns a MoveQuery used to check collision with other entities.
    pub fn check_move(
        &self,
        spatial_area: &mut SpatialHash,
        boundary: (u32, u32),
        _source: (i32, i32),
        trajectory: (f32, f32),
        speed: f32,
    ) -> MoveQuery {
        let (dx, dy) = trajectory;

        // Apply movement deltas within bounds.
        let new_x = (self.rect().x() as f32 + (dx * speed))
            .max(0.0)
            .min((boundary.0 - self.rect().width()) as f32)
            .floor() as i32;
        let new_y = (self.rect().y() as f32 + (dy * speed))
            .max(0.0)
            .min((boundary.1 - self.rect().height()) as f32)
            .floor() as i32;

        // Builds the query.
        let mut query = MoveQuery {
            uuid: self.uuid,
            source: (self.rect().x(), self.rect().y()),
            destination: (new_x, new_y),
            trajectory: (dx, dy),
            entity_size: self.rect().size(),
            nearby: HashSet::new(),
        };

        // Get nearby entities.
        if query.has_moved() {
            let (width, height) = self.rect().size();
            query.nearby = spatial_area.query(new_x, new_y, width, height, Some(self.uuid));
        }

        query
    }

    /// Finalizes the movement utilizing the query. Updates the spatial hash with the new position.
    pub fn move_entity(&mut self, spatial_area: &mut SpatialHash, query: &MoveQuery, size: u16) {
        if !query.is_stuck() {
            self.has_moved = query.has_moved();
            self.last_trajectory = query.trajectory;
            spatial_area.remove_entity(self);
            self.apply_move(query.destination, size);
            self.faced_left = query.destination.0 < self.rect().x();
            spatial_area.insert_entity(self);
        } else {
            self.last_trajectory = (0.0, 0.0);
            self.has_moved = query.has_moved();
        }
    }
}
