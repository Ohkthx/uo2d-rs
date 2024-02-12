use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::entity::{Entity, MoveQuery};
use crate::object::{Object, Position};

#[derive(Default)]
struct Cell {
    entities: HashSet<Uuid>,
}

/// Spatial Hash is used to check locality of entities and check collisions.
pub struct SpatialHash {
    cell_size: i32,
    cells: HashMap<(i32, i32), Cell>,
}

impl SpatialHash {
    /// Creates a new Spatial Hash, the cell_size should be the average size of entities.
    pub fn new(cell_size: i32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    /// Translates coordinates into cell coordinates.
    #[inline]
    fn cell_coords(&self, position: (i32, i32)) -> (i32, i32) {
        (position.0 / self.cell_size, position.1 / self.cell_size)
    }

    /// Adds an entity into a cell, pulling the locational data from it.
    pub fn insert_entity(&mut self, entity: &Entity) {
        let uuid = entity.uuid;

        let (start_x, start_y) = self.cell_coords(entity.object().top_left());
        let (end_x, end_y) = self.cell_coords(entity.object().bottom_right());

        for x in start_x..=end_x {
            for y in start_y..=end_y {
                self.cells.entry((x, y)).or_default().entities.insert(uuid);
            }
        }
    }

    /// Removes an entity from a cell, pulling the locational data from it.
    pub fn remove_entity(&mut self, entity: &Entity) {
        let uuid = entity.uuid;
        let (start_cell_x, start_cell_y) = self.cell_coords(entity.object().top_left());
        let (end_cell_x, end_cell_y) = self.cell_coords(entity.object().bottom_right());

        for x in start_cell_x..=end_cell_x {
            for y in start_cell_y..=end_cell_y {
                if let Some(cell) = self.cells.get_mut(&(x, y)) {
                    cell.entities.remove(&uuid);
                }
            }
        }
    }

    // Queries for UUIDs of entities within the specified rectangle
    pub fn query(&self, entity: &Object, exclude_uuid: Option<Uuid>) -> HashSet<Uuid> {
        let start = self.cell_coords(entity.top_left());
        let end = self.cell_coords(entity.bottom_right());

        let (start_x, start_y) = start;
        let (end_x, end_y) = end;

        let mut result = HashSet::new();
        for cell_x in start_x..=end_x {
            for cell_y in start_y..=end_y {
                if let Some(cell) = self.cells.get(&(cell_x, cell_y)) {
                    for &entity_id in &cell.entities {
                        // Check if the UUID is not the one to be excluded, if any
                        if exclude_uuid.map_or(true, |excl_uuid| entity_id != excl_uuid) {
                            result.insert(entity_id);
                        }
                    }
                }
            }
        }

        result
    }

    /// Get the closest position before collision.
    pub fn till_collision(query: &MoveQuery, entity: &Entity) -> Option<Position> {
        if query.nearby.is_empty() || query.uuid == entity.uuid {
            return Some(query.destination);
        }

        let (sx, sy, sz) = query.source;
        let (mut x, mut y, z) = query.destination;
        let (dx, dy) = query.trajectory;
        let (w, h) = query.entity_size;

        // Correct approach to move back towards the source incrementally
        while entity.object().intersects(&Object::new(x, y, z, w, h)) {
            // Move back one step at a time towards the source
            if dx > 0.0 && x > sx {
                x -= 1;
            } else if dx < 0.0 && x < sx {
                x += 1;
            }

            if dy > 0.0 && y > sy {
                y -= 1;
            } else if dy < 0.0 && y < sy {
                y += 1;
            }

            // Check if we have returned to the source position
            if (x == sx && y == sy) || (dx == 0.0 && dy == 0.0) {
                // If at source or no movement in trajectory, check for collision at source
                if entity.object().intersects(&Object::new(sx, sy, sz, w, h)) {
                    return Some((sx, sy, sz));
                }
                break;
            }
        }

        Some((x, y, z))
    }

    /// The coordinates that can be moved in until a collision is detected.
    pub fn till_collisions(query: &MoveQuery, objects: &HashMap<Uuid, Entity>) -> Option<Position> {
        if query.nearby.is_empty() {
            // If there are no nearby objects, we can move to the destination.
            return Some(query.destination);
        }

        let mut closest_position = query.destination;
        let mut collision_detected = false;

        for uuid in &query.nearby {
            // Skip checking the query object itself.
            if *uuid == query.uuid {
                continue;
            }

            if let Some(entity) = objects.get(uuid) {
                // Use till_collision for each entity to check for collisions.
                match SpatialHash::till_collision(query, entity) {
                    Some(pos) => {
                        // If till_collision returns a position, check if it's closer than the current closest_position.
                        if !collision_detected
                            || SpatialHash::is_closer_to_source(query.source, pos, closest_position)
                        {
                            closest_position = pos;
                            collision_detected = true;
                        }
                    }
                    None => {
                        // If till_collision returns None, it means a collision is unavoidable for this entity.
                        return None;
                    }
                }
            }
        }

        if collision_detected {
            Some(closest_position)
        } else {
            // If no collisions are detected, we can move to the destination.
            Some(query.destination)
        }
    }

    // Helper function to check if a position is closer to the source
    fn is_closer_to_source(source: Position, new_pos: Position, current_pos: Position) -> bool {
        let dist_new = (new_pos.0 - source.0).pow(2) + (new_pos.1 - source.1).pow(2);
        let dist_current = (current_pos.0 - source.0).pow(2) + (current_pos.1 - source.1).pow(2);

        dist_new < dist_current
    }
}
