use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

use crate::object::Position;

pub type Coordinate = (f32, f32);
pub type Bounds = (i32, i32, i32, i32);

fn default_bounding_box() -> Bounds {
    (0, 0, 0, 0)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Region {
    pub name: String,
    pub description: String,
    pub spawn: Position,
    pub file: String,
    vertices: Vec<Coordinate>,
    #[serde(default = "default_bounding_box")]
    bounding_box: Bounds,
}

impl Region {
    // Function to load a map from a YAML file
    pub fn load(file_path: &str) -> Result<Region, serde_yaml::Error> {
        let file_content = std::fs::read_to_string(file_path).expect("Failed to read map file");
        let mut region: Region = serde_yaml::from_str(&file_content)?;

        // Update the bounding box with the vertices.
        region.bounding_box = Self::calc_bounding_box(&region);
        Ok(region)
    }

    /// Checks if a coordinate is within a region using the ray casting algorithm.
    pub fn is_within(&self, position: &Position) -> bool {
        let mut is_inside = false;
        let mut j = self.vertices.len() - 1;
        let (x, y) = (position.0 as f32, position.1 as f32);

        for i in 0..self.vertices.len() {
            let (xi, yi) = (self.vertices[i].0, self.vertices[i].1);
            let (xj, yj) = (self.vertices[j].0, self.vertices[j].1);

            if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
                is_inside = !is_inside;
            }

            j = i;
        }

        is_inside
    }

    /// Checks if an entire object is within bounds by checking all corners.
    pub fn is_inbounds(&self, position: &Position, size: (u16, u16)) -> bool {
        let (pos_x, pos_y, _pos_z) = *position; // Assuming Z is not used for boundary check
        let (width, height) = size;

        // Define all corners of the object
        let corners = [
            *position,
            (pos_x + width as i32, pos_y, _pos_z),
            (pos_x, pos_y + height as i32, _pos_z),
            (pos_x + width as i32, pos_y + height as i32, _pos_z),
        ];

        // Check if all corners are within the region
        corners
            .iter()
            .all(|&corner| self.is_within(&(corner.0, corner.1, corner.2)))
    }

    /// Calculates the bounding box for a region.
    fn calc_bounding_box(region: &Region) -> Bounds {
        let min_x = region
            .vertices
            .iter()
            .map(|v| v.0)
            .fold(f32::INFINITY, |a, b| a.min(b));
        let max_x = region
            .vertices
            .iter()
            .map(|v| v.0)
            .fold(f32::NEG_INFINITY, |a, b| a.max(b));
        let min_y = region
            .vertices
            .iter()
            .map(|v| v.1)
            .fold(f32::INFINITY, |a, b| a.min(b));
        let max_y = region
            .vertices
            .iter()
            .map(|v| v.1)
            .fold(f32::NEG_INFINITY, |a, b| a.max(b));
        (min_x as i32, min_y as i32, max_x as i32, max_y as i32)
    }

    /// Obtains the bounding box for region.
    pub fn bounding_box(&self) -> Bounds {
        self.bounding_box
    }
}

/// Manages the region data for all loaded regions.
pub struct RegionManager {
    regions: HashMap<u8, Region>,
    map: Vec<Vec<u8>>,
}

impl RegionManager {
    /// Loads all region data at launch, initializing the map.
    pub fn new() -> Self {
        let (width, height, regions) = Self::load("assets/regions");

        let mut regions_map: HashMap<u8, Region> = HashMap::new();
        let mut map: Vec<Vec<u8>> = vec![vec![0; height as usize]; width as usize]; // Adjusted for dynamic sizing

        // Adjusted loop to account for regions defined by vertices
        for (id, region) in regions.iter().enumerate() {
            let id = id as u8;
            regions_map.insert(id, region.clone());

            let (min_x, min_y, max_x, max_y) = region.bounding_box();

            // Iterate over each point within the bounding box
            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    if region.is_within(&(x, y, 0))
                        && x >= 0
                        && y >= 0
                        && (x as u32) < width
                        && (y as u32) < height
                    {
                        map[x as usize][y as usize] = id;
                    }
                }
            }
        }

        Self {
            regions: regions_map,
            map,
        }
    }

    /// Finds and returns the Region corresponding to the given Position.
    pub fn get_region(&self, position: &Position) -> Option<&Region> {
        let (pos_x, pos_y, _pos_z) = *position;

        // Ensure the position is within the bounds of the map.
        if pos_x < 0
            || pos_y < 0
            || pos_x as usize >= self.map.len()
            || pos_y as usize >= self.map[0].len()
        {
            return None;
        }

        // Find and return the corresponding region from the regions HashMap.
        self.regions.get(&self.map[pos_x as usize][pos_y as usize])
    }

    /// Loads all regions based on the `.*yaml` file extension.
    fn load(path: &str) -> (u32, u32, Vec<Region>) {
        let mut regions: Vec<Region> = Vec::new();
        let mut max_width = 0;
        let mut max_height = 0;

        for file_path in get_yaml_filenames(Path::new(path)).iter() {
            if let Ok(region) = Region::load(file_path) {
                let (.., max_x, max_y) = region.bounding_box();

                // Update max_width and max_height based on the region's vertices
                if max_x > max_width as i32 {
                    max_width = max_x as u32;
                }
                if max_y > max_height as i32 {
                    max_height = max_y as u32;
                }

                regions.push(region);
            }
        }

        (max_width, max_height, regions)
    }
}

/// Obtains all YAML filenames within a directory.
fn get_yaml_filenames(path: &Path) -> Vec<String> {
    let mut yaml_files = Vec::new();
    if !path.is_dir() {
        return Vec::new();
    }

    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => panic!("Unable to read the region data."),
    };

    for entry in entries.flatten() {
        if let Some(ext) = entry.path().extension() {
            if ext == "yaml" {
                yaml_files.push(entry.path().to_string_lossy().to_string());
            }
        }
    }

    yaml_files
}
