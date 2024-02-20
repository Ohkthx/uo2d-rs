use serde::{Deserialize, Deserializer, Serialize};

use super::{sort_coordinates_clockwise, Bounds, Vec2, Vec3};

/// Allows an object to be transformed.
#[derive(Clone, Debug, Serialize, Default)]
pub struct Transform {
    vertices: Vec<Vec3>,
    bounding_box: Bounds,
    layer: f64,
}

impl Transform {
    /// Creates a new transform from vertices.
    pub fn from_vertices(vertices: &[Vec3]) -> Self {
        let layer = vertices
            .iter()
            .map(|vec| vec.z())
            .fold(f64::INFINITY, |a, b| a.min(b))
            .min(0.0);

        Self {
            vertices: sort_coordinates_clockwise(vertices),
            bounding_box: Bounds::from_vertices(vertices),
            layer,
        }
    }

    /// Creates a new transform from bounds.
    pub fn from_bounds(bounds: Bounds) -> Self {
        Self {
            vertices: bounds.as_coords(),
            bounding_box: bounds,
            layer: bounds.z(),
        }
    }

    /// Creates a new transform from a coordinate and size. Coordinate is top-left.
    pub fn from_vecs(coord: Vec3, size: Vec2) -> Self {
        Self::from_bounds(Bounds::from_vec(coord, size))
    }

    /// Moves the transform to the new position.
    pub fn set_position(&mut self, coord: &Vec3) {
        // Assuming position() calculates the centroid or a representative point
        let current = self.position();
        let x_offset = coord.x() - current.x();
        let y_offset = coord.y() - current.y();

        // Move all of the internal vertices by the calculated offset
        for vertex in &mut self.vertices {
            vertex.set_x(vertex.x() + x_offset);
            vertex.set_y(vertex.y() + y_offset);
        }

        // Recalculate the bounding box based on the new vertex positions
        self.bounding_box = Bounds::from_vertices(&self.vertices);
    }

    /// Applies a velocity where bounds is the limitation, returning a new transform.
    pub fn applied_velocity(&self, velocity: &Vec2, bounds: &Bounds) -> Self {
        let step_size = 1.0;
        let mut vel = *velocity;
        let (x, y, z) = self.position().as_tuple();
        let (width, height) = self.bounding_box().dimensions().as_tuple();

        while vel != Vec2::ORIGIN {
            let (mod_x, mod_y) = (x + vel.x(), y + vel.y());
            // Generate test positions for the entity's corners at the tentative position.
            let test_positions = [
                Vec3::new(mod_x, mod_y, z),
                Vec3::new(mod_x + width, mod_y, z),
                Vec3::new(mod_x, mod_y + height, z),
                Vec3::new(mod_x + width, mod_y + height, z),
            ];

            // Check if all corners of the entity at the tentative position are within the region.
            if test_positions
                .iter()
                .all(|&pos| bounds.coord_within_2d(&pos))
            {
                let mut new = self.clone();
                new.set_position(&test_positions[0]);
                return new;
            }

            // Move towards origin (no velocity)
            vel = vel.towards_origin(step_size);
        }

        // Closest is not moving.
        self.clone()
    }

    /// Obtains the bounding box for region.
    pub fn bounding_box(&self) -> Bounds {
        self.bounding_box
    }

    /// Gets the top-left bounding box position that represents this transform.
    pub fn position(&self) -> Vec3 {
        self.bounding_box().top_left_3d()
    }

    /// Determines if a point is inside the polygon defined by the transform's vertices.
    pub fn coord_within(&self, coord: &Vec3) -> bool {
        let mut inside = false;
        let mut j = self.vertices.len() - 1;

        for i in 0..self.vertices.len() {
            let xi = self.vertices[i].x();
            let yi = self.vertices[i].y();
            let xj = self.vertices[j].x();
            let yj = self.vertices[j].y();

            let intersect = ((yi > coord.y()) != (yj > coord.y()))
                && (coord.x() < (xj - xi) * (coord.y() - yi) / (yj - yi) + xi);

            if intersect {
                inside = !inside;
            }

            j = i;
        }

        inside
    }

    // Checks if this transform intersects with another by checking edge intersections.
    #[allow(dead_code)]
    pub fn intersects(&self, other: &Self) -> bool {
        // First check if bounding boxes intersect. This is cheap.
        if !self.bounding_box.intersects_3d(&other.bounding_box) {
            return false;
        }

        // Check for any edge intersection between the two polygons.
        for i in 0..self.vertices.len() {
            let a1 = self.vertices[i];
            let a2 = self.vertices[(i + 1) % self.vertices.len()];

            for j in 0..other.vertices.len() {
                let b1 = other.vertices[j];
                let b2 = other.vertices[(j + 1) % other.vertices.len()];

                if lines_intersect(a1, a2, b1, b2) {
                    return true;
                }
            }
        }

        // Verify if one polygon is entirely inside the other.
        if self.coord_within(&other.vertices[0]) || other.coord_within(&self.vertices[0]) {
            return true;
        }

        false
    }
}

impl<'de> Deserialize<'de> for Transform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(Vec<Vec3>);

        let helper = Helper::deserialize(deserializer)?;
        Ok(Transform::from_vertices(&helper.0))
    }
}

/// Determines if the line segments (a1, a2) and (b1, b2) intersect.
fn lines_intersect(a1: Vec3, a2: Vec3, b1: Vec3, b2: Vec3) -> bool {
    // Calculate direction of the lines
    let d1 = (a2.x() - a1.x(), a2.y() - a1.y());
    let d2 = (b2.x() - b1.x(), b2.y() - b1.y());

    let denominator = d1.0 * d2.1 - d2.0 * d1.1;

    // Lines are parallel if denominator is 0
    if denominator.abs() < f64::EPSILON {
        return false;
    }

    let ua = ((d2.0 * (a1.y() - b1.y()) - d2.1 * (a1.x() - b1.x())) / denominator).abs();
    let ub = ((d1.0 * (a1.y() - b1.y()) - d1.1 * (a1.x() - b1.x())) / denominator).abs();

    // If ua and ub are both between 0 and 1, lines intersect
    ua <= 1.0 && ub <= 1.0 && ua >= 0.0 && ub >= 0.0
}
