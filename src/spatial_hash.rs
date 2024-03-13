use std::collections::{HashMap, HashSet};

use crate::components::{Bounds, Position, Vec2, Vec3};
use crate::ecs::Entity;
use crate::server::systems::movement::MoveQuery;

#[derive(Default)]
struct Cell {
    entities: HashSet<Entity>,
}

/// Spatial Hash is used to check locality of entities and check collisions.
pub struct SpatialHash {
    cell_size: usize,
    cells: HashMap<(usize, usize), Cell>,
}

impl SpatialHash {
    /// Creates a new Spatial Hash, the cell_size should be the average size of entities.
    pub fn new(cell_size: usize) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    /// Translates coordinates into cell coordinates.
    #[inline]
    fn cell_coords(&self, position: Vec2) -> (usize, usize) {
        (
            position.x() as usize / self.cell_size,
            position.y() as usize / self.cell_size,
        )
    }

    /// Adds an entity into a cell, pulling the locational data from it.
    pub fn insert_object(&mut self, entity: &Entity, obj: &Bounds) {
        let (start_x, start_y) = self.cell_coords(obj.top_left_2d());
        let (end_x, end_y) = self.cell_coords(obj.bottom_right_2d());

        for x in start_x..=end_x {
            for y in start_y..=end_y {
                self.cells
                    .entry((x, y))
                    .or_default()
                    .entities
                    .insert(*entity);
            }
        }
    }

    /// Removes an entity from a cell, pulling the locational data from it.
    pub fn remove_object(&mut self, entity: &Entity, obj: &Bounds) {
        let (start_cell_x, start_cell_y) = self.cell_coords(obj.top_left_2d());
        let (end_cell_x, end_cell_y) = self.cell_coords(obj.bottom_right_2d());

        for x in start_cell_x..=end_cell_x {
            for y in start_cell_y..=end_cell_y {
                if let Some(cell) = self.cells.get_mut(&(x, y)) {
                    cell.entities.remove(entity);
                }
            }
        }
    }

    // Queries for entities of entities within the specified rectangle
    pub fn query(&self, bounds: &Bounds, exclude_entity: Option<&Entity>) -> HashSet<Entity> {
        let start = self.cell_coords(bounds.top_left_2d());
        let end = self.cell_coords(bounds.bottom_right_2d());

        let (start_x, start_y) = start;
        let (end_x, end_y) = end;

        let mut result = HashSet::new();
        for cell_x in start_x..=end_x {
            for cell_y in start_y..=end_y {
                if let Some(cell) = self.cells.get(&(cell_x, cell_y)) {
                    for &entity_id in &cell.entities {
                        // Check if the entity is not the one to be excluded, if any
                        if exclude_entity.map_or(true, |excl_entity| entity_id != *excl_entity) {
                            result.insert(entity_id);
                        }
                    }
                }
            }
        }

        result
    }

    pub fn till_collision(query: &MoveQuery, bounds: &Bounds, step: f64) -> Option<Vec3> {
        if query.nearby.is_empty() {
            // If there are no nearby objects, the path to the destination is clear.
            return Some(query.destination);
        }

        // Extract initial positions and size.
        let (sx, sy, _) = query.source.as_tuple();
        let (mut dx, mut dy, dz) = query.destination.as_tuple();
        let (vel_x, vel_y) = query.velocity.as_tuple();
        let (w, h) = query.entity_size.as_tuple();

        // If the destination does not intersect with bounds, return it directly.
        if !bounds.intersects_3d(&Bounds::new(dx, dy, dz, w, h)) {
            return Some(Vec3::new(dx, dy, dz));
        }

        // Calculate the step size for backtracking based on velocity direction.
        let step_x = vel_x.signum() * step;
        let step_y = vel_y.signum() * step;

        while bounds.intersects_3d(&Bounds::new(dx, dy, dz, w, h)) {
            // Move back towards the source position incrementally, based on the direction of the velocity.
            if vel_x != 0.0 {
                dx -= step_x;
            }
            if vel_y != 0.0 {
                dy -= step_y;
            }

            // Check if the position has moved back to or past the source; if so, break the loop.
            if (vel_x > 0.0 && dx <= sx)
                || (vel_x < 0.0 && dx >= sx)
                || (vel_y > 0.0 && dy <= sy)
                || (vel_y < 0.0 && dy >= sy)
            {
                break;
            }
        }

        // After adjusting, if we're still in bounds or have returned to the source, return None.
        if bounds.intersects_3d(&Bounds::new(dx, dy, dz, w, h)) || (dx == sx && dy == sy) {
            return None;
        }

        // Return the adjusted position if a collision-free spot is found.
        Some(Vec3::new(dx, dy, dz))
    }

    /// The coordinates that can be moved in until a collision is detected.
    pub fn till_collisions(
        query: &MoveQuery,
        objects: &HashMap<Entity, &Position>,
        step: f64,
    ) -> Option<Vec3> {
        if query.nearby.is_empty() {
            // If there are no nearby objects, we can move to the destination.
            return Some(query.destination);
        }

        let mut closest_position = query.destination;
        let mut collision_detected = false;

        for entity in &query.nearby {
            // Skip checking the query object itself.
            if *entity == query.entity {
                continue;
            }

            if let Some(entity) = objects.get(entity) {
                let bounds = Bounds::from_vec(entity.loc, entity.size);
                // Use till_collision for each entity to check for collisions.
                match SpatialHash::till_collision(query, &bounds, step) {
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
    fn is_closer_to_source(source: Vec3, new_pos: Vec3, current_pos: Vec3) -> bool {
        let dist_new = new_pos.distance_2d(&source);
        let dist_current = current_pos.distance_2d(&source);
        dist_new < dist_current
    }
}
