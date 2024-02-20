use std::{collections::HashMap, path::Path};

use serde::Deserialize;

use crate::components::{Bounds, Transform, Vec3};
use crate::sprintln;

#[derive(Debug, Deserialize, Clone)]
pub struct Region {
    pub name: String,
    pub description: String,
    pub spawn: Vec3,
    pub file: String,
    #[serde(rename = "vertices")]
    transform: Transform,
}

impl Region {
    // Function to load a map from a YAML file
    pub fn load(file_path: &str) -> Result<Region, serde_yaml::Error> {
        let file_content = std::fs::read_to_string(file_path).expect("Failed to read map file");
        serde_yaml::from_str(&file_content)
    }

    /// Checks if a coordinate is within a region using the ray casting algorithm.
    pub fn is_within(&self, position: &Vec3) -> bool {
        self.transform.coord_within(position)
    }

    /// Checks if an entire object is within bounds by checking all corners.
    pub fn is_inbounds(&self, bounds: &Bounds) -> bool {
        self.bounding_box().contains_2d(bounds)
    }

    /// Obtains the bounding box for region.
    pub fn bounding_box(&self) -> Bounds {
        self.transform.bounding_box()
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

            let (min_x, min_y, _) = region.bounding_box().top_left_3d().as_tuple();
            let (max_x, max_y, _) = region.bounding_box().bottom_right_3d().as_tuple();
            let (width, height) = region.bounding_box().dimensions().as_tuple();

            // Iterate over each row in the bounding box, adjusting the range as needed
            for (x, row) in map
                .iter_mut()
                .enumerate()
                .take(max_x as usize + 1)
                .skip(min_x as usize)
            {
                // Iterate over each column in the row within the bounding box
                for (y, cell) in row
                    .iter_mut()
                    .enumerate()
                    .take(max_y as usize + 1)
                    .skip(min_y as usize)
                {
                    if region.is_within(&Vec3::new(x as f64, y as f64, 0.0))
                        && x < width as usize
                        && y < height as usize
                    {
                        // Directly modify the cell as needed
                        *cell = id;
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
    pub fn get_region(&self, coord: &Vec3) -> Option<&Region> {
        let (x, y, _z) = coord.as_tuple();

        // Ensure the position is within the bounds of the map.
        if x < 0. || y < 0. || x as usize >= self.map.len() || y as usize >= self.map[0].len() {
            return None;
        }

        // Find and return the corresponding region from the regions HashMap.
        self.regions.get(&self.map[x as usize][y as usize])
    }

    /// Loads all regions based on the `.*yaml` file extension.
    fn load(path: &str) -> (f64, f64, Vec<Region>) {
        let mut regions: Vec<Region> = Vec::new();
        let mut max_width = 0.;
        let mut max_height = 0.;

        for file_path in get_yaml_filenames(Path::new(path)).iter() {
            match Region::load(file_path) {
                Ok(region) => {
                    let (max_x, max_y, _) = region.bounding_box().bottom_right_3d().as_tuple();

                    // Update max_width and max_height based on the region's vertices
                    if max_x > max_width {
                        max_width = max_x;
                    }
                    if max_y > max_height {
                        max_height = max_y;
                    }

                    regions.push(region);
                }
                Err(why) => sprintln!("Error while loading {}: {}", file_path, why),
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
