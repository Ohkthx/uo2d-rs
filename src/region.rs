use serde::{Deserialize, Serialize};

use crate::object::Position;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Boundary {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Region {
    pub name: String,
    pub description: String,
    pub spawn: Position,
    pub file: String,
    pub boundaries: Boundary,
}

impl Region {
    // Function to load a map from a YAML file
    pub fn load(file_path: &str) -> Result<Region, serde_yaml::Error> {
        let file_content = std::fs::read_to_string(file_path).expect("Failed to read map file");
        serde_yaml::from_str(&file_content)
    }

    /// Checks if a coordinate is within a region.
    pub fn is_within(&self, position: &Position) -> bool {
        if position.0 < self.boundaries.x || position.0 >= self.boundaries.width as i32 {
            false
        } else {
            !(position.1 < self.boundaries.y || position.1 >= self.boundaries.height as i32)
        }
    }

    /// Checks if an entire object is within bounds.
    pub fn is_inbounds(&self, position: &Position, size: (u16, u16)) -> bool {
        let (pos_x, pos_y, pos_z) = *position;
        let (width, height) = size;

        // Calculate the bottom right corner.
        let bottom_right = (pos_x + width as i32, pos_y + height as i32, pos_z);

        self.is_within(position) && self.is_within(&bottom_right)
    }
}
