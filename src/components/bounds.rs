use serde::{Deserialize, Serialize};

use super::{Vec2, Vec3};

/// Boundary box, also just a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Bounds {
    data: Vec3,
    width: f64,
    height: f64,
}

impl Bounds {
    pub fn new(x: f64, y: f64, z: f64, width: f64, height: f64) -> Self {
        Self {
            data: Vec3::new(x, y, z),
            width,
            height,
        }
    }

    pub fn from_vec(coord: Vec3, size: Vec2) -> Self {
        Self {
            data: coord,
            width: size.x(),
            height: size.y(),
        }
    }

    #[allow(dead_code)]
    pub fn set_x(&mut self, value: f64) {
        self.data.set_x(value)
    }

    #[allow(dead_code)]
    pub fn set_y(&mut self, value: f64) {
        self.data.set_y(value)
    }

    #[allow(dead_code)]
    pub fn set_z(&mut self, value: f64) {
        self.data.set_z(value)
    }

    #[inline]
    pub fn x(&self) -> f64 {
        self.data.x()
    }

    #[inline]
    pub fn y(&self) -> f64 {
        self.data.y()
    }

    #[inline]
    pub fn z(&self) -> f64 {
        self.data.z()
    }

    #[inline]
    pub fn width(&self) -> f64 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn dimensions(&self) -> Vec2 {
        Vec2::new(self.width(), self.height())
    }

    /// Calculates the bounds from vertices and makes it public.
    pub fn from_vertices(vertices: &[Vec3]) -> Self {
        let min_x = vertices.iter().map(Vec3::x).fold(f64::INFINITY, f64::min);
        let max_x = vertices
            .iter()
            .map(Vec3::x)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = vertices.iter().map(Vec3::y).fold(f64::INFINITY, f64::min);
        let max_y = vertices
            .iter()
            .map(Vec3::y)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_z = vertices.iter().map(Vec3::z).fold(f64::INFINITY, f64::min);

        Self {
            data: Vec3::new(min_x, min_y, min_z),
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    /// Converts a bounds to a series of vertices.
    pub fn as_coords(&self) -> Vec<Vec3> {
        sort_coordinates_clockwise(&[
            Vec3::new(self.x(), self.y(), self.z()),
            Vec3::new(self.x() + self.width(), self.y(), self.z()),
            Vec3::new(self.x() + self.width(), self.y() + self.height(), self.z()),
            Vec3::new(self.x(), self.y() + self.height(), self.z()),
        ])
    }

    /// Obtains the center point of the bounds.
    pub fn center_2d(&self) -> Vec2 {
        Vec2::new(
            self.x() + (self.width() / 2.),
            self.y() + (self.height() / 2.),
        )
    }

    /// Obtains the top-left coordinate for the bounds.
    pub fn top_left_2d(&self) -> Vec2 {
        Vec2::new(self.x(), self.y())
    }

    /// Obtains the bottom-right coordinate for the bounds.
    pub fn bottom_right_2d(&self) -> Vec2 {
        Vec2::new(self.x() + self.width, self.y() + self.height())
    }

    /// Obtains the top-left coordinate for the bounds.
    pub fn top_left_3d(&self) -> Vec3 {
        Vec3::new(self.x(), self.y(), self.z())
    }

    /// Obtains the bottom-right coordinate for the bounds.
    pub fn bottom_right_3d(&self) -> Vec3 {
        Vec3::new(self.x() + self.width, self.y() + self.height(), self.z())
    }

    /// Checks if a coordinate is within the bounds, assuming exclusive upper bounds.
    pub fn coord_within_2d(&self, coord: &Vec3) -> bool {
        (self.x() <= coord.x() && coord.x() <= self.x() + self.width)
            && (self.y() <= coord.y() && coord.y() <= self.y() + self.height)
    }

    /// Checks if a coordinate is within the bounds, assuming exclusive upper bounds.
    #[allow(dead_code)]
    pub fn coord_within_3d(&self, coord: &Vec3) -> bool {
        if coord.z() != self.z() {
            return false;
        }

        self.coord_within_2d(coord)
    }

    /// Checks if this bounds completely contains another.
    #[allow(dead_code)]
    pub fn contains_2d(&self, other: &Self) -> bool {
        // Check if the top-left corner of other is inside self.
        let top_left_inside = self.x() <= other.x() && self.y() <= other.y();

        // Check if the bottom-right corner of other is inside self.
        let bottom_right_inside = (self.x() + self.width) >= (other.x() + other.width)
            && (self.y() + self.height) >= (other.y() + other.height);

        top_left_inside && bottom_right_inside
    }

    /// Checks if this bounds intersects with another.
    pub fn intersects_2d(&self, other: &Self) -> bool {
        // Check if one bounding box is to the left of the other.
        if self.x() + self.width <= other.x() || other.x() + other.width <= self.x() {
            return false;
        }

        // Check if one bounds is above the other.
        if self.y() + self.height <= other.y() || other.y() + other.height <= self.y() {
            return false;
        }

        true
    }

    /// Checks if this bounds intersects with another.
    pub fn intersects_3d(&self, other: &Self) -> bool {
        if other.z() != self.z() {
            return false;
        }

        self.intersects_2d(other)
    }

    /// Returns a scaled version of the bounds, it is scaled from the center..
    pub fn scaled_center(&self, scalar: f64) -> Bounds {
        let new_width = self.width * scalar;
        let new_height = self.height * scalar;

        // Calculate how much the width and height have increased.
        let width_increase = new_width - self.width;
        let height_increase = new_height - self.height;

        // Shift x and y to adjust for the increase in size, to keep the center the same.
        let x = self.x() - width_increase / 2.;
        let y = self.y() - height_increase / 2.;

        // Create a bounds.
        Bounds::new(x, y, self.z(), new_width, new_height)
    }

    /// Clamps another bounding box within.
    pub fn clamp_within(&self, other: &Self) -> Self {
        if other.width > self.width || other.height > self.height {
            return *other;
        }

        // Attempt to clamp `other` within `self`.
        let clamped_x = other
            .x()
            .clamp(self.x(), self.x() + self.width() - other.width());
        let clamped_y = other
            .y()
            .clamp(self.y(), self.y() + self.height() - other.height());

        Bounds::new(clamped_x, clamped_y, other.z(), other.width, other.height)
    }

    /// Clamps a vec3 within the bounds.
    pub fn clamp_coord_within(&self, coord: Vec3) -> Vec3 {
        let x = coord.x().clamp(self.x(), self.x() + self.width());
        let y = coord.y().clamp(self.y(), self.y() + self.height());
        Vec3::new(x, y, coord.z())
    }
}

/// Sorts coordinates in clockwise order around their centroid.
pub(crate) fn sort_coordinates_clockwise(coordinates: &[Vec3]) -> Vec<Vec3> {
    // Clone the input vector to not modify the original
    let mut sorted_coordinates = coordinates.to_vec();

    // Step 1: Find a pivot, here we use the point with the lowest y-coordinate.
    // In case of a tie, the point with the lowest x-coordinate.
    let pivot = *sorted_coordinates
        .iter()
        .min_by(|a, b| {
            a.y()
                .partial_cmp(&b.y())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.x().partial_cmp(&b.x()).unwrap())
        })
        .expect("Cannot find a minimum in an empty list");

    // Step 2: Sort the points based on their angle from the pivot
    sorted_coordinates.sort_by(|a, b| {
        pivot
            .pivot_angle(a)
            .partial_cmp(&pivot.pivot_angle(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    sorted_coordinates
}
