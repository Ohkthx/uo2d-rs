/// Position containing an x, y, and z-axis.
pub type Position = (i32, i32, i8);

/// Represents and object that takes up space and has a z-axis (layer).
pub struct Object {
    /// Left / Right coord.
    x: i32,
    /// Up / Down coord.
    y: i32,
    /// Layer object exists on.
    z: i8,
    /// Width of the object.
    w: u16,
    /// Height of the object.
    h: u16,
}

impl Object {
    pub fn new(x: i32, y: i32, z: i8, w: u16, h: u16) -> Self {
        Self { x, y, z, w, h }
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
        self.w
    }

    #[inline]
    pub fn width_u32(&self) -> u32 {
        self.width() as u32
    }

    #[inline]
    pub fn height(&self) -> u16 {
        self.h
    }

    #[inline]
    pub fn height_u32(&self) -> u32 {
        self.height() as u32
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

    /// Updates the position of the object.
    pub fn update_position(&mut self, position: (i32, i32, i8)) {
        self.x = position.0;
        self.y = position.1;
        self.z = position.2;
    }

    /// Updates the entire object.
    pub fn update(&mut self, position: (i32, i32, i8), width: u16, height: u16) {
        self.update_position(position);
        if width > self.w {
            self.w = width;
        }

        if height > self.h {
            self.h = height;
        }
    }

    // Checks if this position intersects with another position
    pub fn intersects(&self, other: &Object) -> bool {
        // Check if they are on the same layer.
        if self.z != other.z {
            return false;
        }

        // Check if one rectangle is to the left of the other
        if self.x + self.w as i32 <= other.x || other.x + other.w as i32 <= self.x {
            return false;
        }

        // Check if one rectangle is above the other
        if self.y + self.h as i32 <= other.y || other.y + other.h as i32 <= self.y {
            return false;
        }

        // If neither condition is true, the rectangles intersect
        true
    }
}
