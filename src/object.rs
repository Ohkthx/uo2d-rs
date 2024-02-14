use serde::{Deserialize, Serialize};

/// Position containing an x, y, and z-axis.
pub type Position = (i32, i32, i8);

/// Represents and object that takes up space and has a z-axis (layer).
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Object {
    /// Left / Right coord.
    x: i32,
    /// Up / Down coord.
    y: i32,
    /// Layer object exists on.
    z: i8,
    /// Width of the object.
    width: u16,
    /// Height of the object.
    height: u16,
}

impl Object {
    pub fn new(x: i32, y: i32, z: i8, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            z,
            width,
            height,
        }
    }

    #[inline]
    pub fn x(&self) -> i32 {
        self.x
    }

    #[inline]
    pub fn y(&self) -> i32 {
        self.y
    }

    #[inline]
    pub fn z(&self) -> i8 {
        self.z
    }

    #[inline]
    pub fn position(&self) -> Position {
        (self.x(), self.y(), self.z())
    }

    #[inline]
    pub fn width(&self) -> u16 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u16 {
        self.height
    }

    #[inline]
    pub fn size(&self) -> (u16, u16) {
        (self.width(), self.height())
    }

    #[inline]
    pub fn top_left(&self) -> (i32, i32) {
        (self.x(), self.y())
    }

    #[inline]
    pub fn bottom_right(&self) -> (i32, i32) {
        (
            self.x() + self.width() as i32,
            self.y() + self.height() as i32,
        )
    }

    #[inline]
    pub fn center(&self) -> (i32, i32) {
        (
            self.x() + (self.width() as i32 / 2),
            self.y() + (self.height() as i32 / 2),
        )
    }

    /// Updates the position of the object.
    pub fn update_position(&mut self, position: (i32, i32, i8)) {
        self.x = position.0;
        self.y = position.1;
        self.z = position.2;
    }

    /// Updates the entire object.
    pub fn update(&mut self, position: (i32, i32, i8), width: u16, height: u16) {
        self.update_position(position);
        if width > self.width {
            self.width = width;
        }

        if height > self.height {
            self.height = height;
        }
    }

    /// Get the range for nearby objects.
    pub fn range(&self, scalar: u16) -> Object {
        let new_width = self.width * scalar;
        let new_height = self.height * scalar;

        // Calculate how much the width and height have increased.
        let width_increase = new_width as i32 - self.width as i32;
        let height_increase = new_height as i32 - self.height as i32;

        // Shift x and y to adjust for the increase in size, to keep the center the same.
        let x = self.x - width_increase / 2;
        let y = self.y - height_increase / 2;

        // Create a new Object with the updated dimensions and position.
        Object::new(x, y, self.z, new_width, new_height)
    }

    // Checks if this position intersects with another position.
    pub fn intersects(&self, other: &Object) -> bool {
        // Check if they are on the same layer.
        if self.z != other.z {
            return false;
        }

        // Check if one rectangle is to the left of the other.
        if self.x + self.width as i32 <= other.x || other.x + other.width as i32 <= self.x {
            return false;
        }

        // Check if one rectangle is above the other.
        if self.y + self.height as i32 <= other.y || other.y + other.height as i32 <= self.y {
            return false;
        }

        true
    }

    /// Gets the nearest coordinates that an object of `size` can exist in relation to the current object at the specified trajectory.
    pub fn place_outside(
        &self,
        trajectory: (f32, f32),
        size: (u16, u16),
        layer: i8,
    ) -> (i32, i32, i8) {
        let (dx, dy) = trajectory;
        let center_a = self.center();
        let (width_b, height_b) = size;

        // Normalize the trajectory.
        let magnitude = (dx.powi(2) + dy.powi(2)).sqrt();
        let (norm_dx, norm_dy) = if magnitude == 0.0 {
            (0.0, 0.0)
        } else {
            (dx / magnitude, dy / magnitude)
        };

        // Calculate distance to place B outside of A considering the sizes of A and B.
        let distance_x = (self.width() as f32 / 2.0 + width_b as f32 / 2.0) * norm_dx.abs();
        let distance_y = (self.height() as f32 / 2.0 + height_b as f32 / 2.0) * norm_dy.abs();

        // Calculate the new top-left position for B
        let new_x = center_a.0 as f32 + norm_dx * distance_x;
        let new_y = center_a.1 as f32 + norm_dy * distance_y;

        // Adjust the position to ensure B's top-left corner is correctly placed.
        let final_x = if dx >= 0.0 {
            new_x.ceil() as i32
        } else {
            new_x.floor() as i32 - width_b as i32
        };
        let final_y = if dy >= 0.0 {
            new_y.ceil() as i32
        } else {
            new_y.floor() as i32 - height_b as i32
        };

        (final_x, final_y, layer)
    }
}
