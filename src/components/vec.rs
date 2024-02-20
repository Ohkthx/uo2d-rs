use serde::{Deserialize, Serialize};

/// Represents 2 dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vec2([f64; 2]);

impl Vec2 {
    /// Represents a Vec2 at the origin.
    pub const ORIGIN: Self = Self([0., 0.]);

    pub fn new(x: f64, y: f64) -> Self {
        Self([x, y])
    }

    pub fn set_x(&mut self, value: f64) {
        self.0[0] = value
    }

    pub fn set_y(&mut self, value: f64) {
        self.0[1] = value
    }

    pub fn x(&self) -> f64 {
        self.0[0]
    }

    pub fn y(&self) -> f64 {
        self.0[1]
    }

    /// Deconstructs the coordinate into a tuple form.
    pub fn as_tuple(&self) -> (f64, f64) {
        (self.x(), self.y())
    }

    /// Calculates the distance between two coordinates, excluding the z-axis.
    pub fn distance(&self, other: &Self) -> f64 {
        f64::sqrt((self.x() - other.x()).powi(2) + (self.y() - other.y()).powi(2))
    }

    /// Gets the length of the vector.
    pub fn length(&self) -> f64 {
        self.distance(&Vec2::ORIGIN)
    }

    // Method to normalize the vector.
    pub fn normalize(&self) -> Vec2 {
        let length = self.length();
        if length != 0.0 {
            Vec2::new(self.x() / length, self.y() / length)
        } else {
            Vec2::new(0.0, 0.0) // Cannot normalize a zero-length vector; return it unchanged.
        }
    }

    /// Method to scale the vector to a specific length.
    pub fn scaled(&self, new_length: f64) -> Vec2 {
        let normalized = self.normalize();
        Vec2::new(normalized.x() * new_length, normalized.y() * new_length)
    }

    /// Applies a scalar to the vector.
    pub fn apply_scalar(&self, scalar: f64) -> Vec2 {
        Vec2::new(self.x() * scalar, self.y() * scalar)
    }

    /// Difference between the current another.
    pub fn offset_from(&self, other: &Self) -> Vec2 {
        Vec2::new(self.x() - other.x(), self.y() - other.y())
    }

    /// Returns a clamped version of the vector based on its magnitude / length.
    pub fn clamped(&self, min_length: f64, max_length: f64) -> Vec2 {
        let length = self.length();
        if length < min_length {
            self.scaled(min_length)
        } else if length > max_length {
            self.scaled(max_length)
        } else {
            *self
        }
    }

    /// Moves a `step` towards origin.
    pub fn towards_origin(&self, step: f64) -> Self {
        let length = self.length();
        if length == 0.0 || step >= length {
            return Vec2::ORIGIN; // Already at the origin or step exceeds distance to origin.
        }

        // Calculate the scaling factor to reduce the vector's length by `step`
        let scale = (length - step) / length;

        Vec2::new(self.x() * scale, self.y() * scale)
    }
}

impl Default for Vec2 {
    fn default() -> Self {
        Self::ORIGIN
    }
}

/// Represents 3 dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vec3([f64; 3]);

impl Vec3 {
    /// Represents a Vec3 at the origin.
    pub const ORIGIN: Self = Self([0., 0., 0.]);

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self([x, y, z])
    }

    pub fn x(&self) -> f64 {
        self.0[0]
    }

    pub fn y(&self) -> f64 {
        self.0[1]
    }

    pub fn z(&self) -> f64 {
        self.0[2]
    }

    pub fn set_x(&mut self, value: f64) {
        self.0[0] = value
    }

    pub fn set_y(&mut self, value: f64) {
        self.0[1] = value
    }

    pub fn set_z(&mut self, value: f64) {
        self.0[2] = value
    }

    /// Deconstructs the coordinate into a tuple form.
    pub fn as_tuple(&self) -> (f64, f64, f64) {
        (self.x(), self.y(), self.z())
    }

    /// Deconstructs the coordinate into an array form.
    pub fn as_vec(&self) -> [f64; 3] {
        self.0
    }

    pub fn from_vec2(vec: Vec2, z: f64) -> Self {
        Vec3::new(vec.x(), vec.y(), z)
    }

    /// Converts to a flat Vec2, removing the z-axis.
    pub fn as_vec2(&self) -> Vec2 {
        Vec2::new(self.x(), self.y())
    }

    /// Calculate the angle of pivot.
    pub fn pivot_angle(&self, other: &Self) -> f64 {
        (self.y() - other.y()).atan2(self.x() - other.x())
    }

    /// Obtains the rounded value for the vector.
    pub fn round(&self) -> Vec3 {
        Vec3::new(self.x().round(), self.y().round(), self.z().round())
    }

    /// Difference between the current another.
    pub fn offset_from_2d(&self, other: &Self) -> Vec3 {
        Vec3::new(self.x() - other.x(), self.y() - other.y(), self.z())
    }

    /// Calculates the distance between two coordinates, excluding the z-axis.
    pub fn distance_2d(&self, other: &Self) -> f64 {
        f64::sqrt((self.x() - other.x()).powi(2) + (self.y() - other.y()).powi(2))
    }

    /// Moves a `step` towards origin.
    #[allow(dead_code)]
    pub fn towards_origin(&self, step: f64) -> Self {
        Self([
            // Calculate new x
            if self.x().abs() <= step {
                0.0
            } else {
                self.x() - step.signum() * step
            },
            // Calculate new y
            if self.y().abs() <= step {
                0.0
            } else {
                self.y() - step.signum() * step
            },
            // Calculate new z
            if self.z().abs() <= step {
                0.0
            } else {
                self.z() - step.signum() * step
            },
        ])
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self::ORIGIN
    }
}
