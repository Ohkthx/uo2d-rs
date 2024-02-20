use std::collections::HashMap;

use sdl2::render::WindowCanvas;

use uuid::Uuid;

use crate::components::{Vec2, Vec3};
use crate::entities::{Camera, Mobile, MobileType};
use crate::timer::TimerManager;

/// Current tracked state of the game.
pub struct Gamestate {
    pub timers: TimerManager,
    locations: HashMap<Uuid, i8>,
    pub entities: HashMap<i8, HashMap<Uuid, Mobile>>,
    pub kill: bool,
}

impl Gamestate {
    /// Initializes the gamestate.
    pub fn new() -> Self {
        Self {
            timers: TimerManager::new(),
            locations: HashMap::new(),
            entities: HashMap::new(),
            kill: false,
        }
    }

    // Get an entity by its UUID
    pub fn get_entity(&self, uuid: &Uuid) -> Option<&Mobile> {
        if let Some(layer) = self.locations.get(uuid) {
            if let Some(entities) = self.entities.get(layer) {
                return entities.get(uuid);
            }
        }
        None
    }

    /// Updates an entity's position and size, if it exists, or inserts a new entity.
    pub fn upsert_entity(&mut self, uuid: Uuid, position: Vec3, size: Vec2) {
        // Create or update the entity with its new or existing color.
        let entity = Mobile::new(uuid, position, size, MobileType::Creature);

        // Assign entity to the new layer and update locations mapping.
        self.locations.insert(uuid, position.z() as i8);
        self.entities
            .entry(position.z() as i8)
            .or_default()
            .insert(uuid, entity);
    }

    pub fn remove_entity(&mut self, uuid: &Uuid) {
        // First, find the layer the entity is in using the locations map and remove the entry.
        if let Some(layer) = self.locations.remove(uuid) {
            // Then, access the sub-map for the layer and attempt to remove the entity by its UUID.
            if let Some(entities) = self.entities.get_mut(&layer) {
                entities.remove(uuid);

                // Remove the layer if it is empty.
                if entities.is_empty() {
                    self.entities.remove(&layer);
                }
            }
        }
    }

    /// Draws all currently stored entities.
    pub fn draw(&self, canvas: &mut WindowCanvas, camera: &Camera) {
        let mut layers: Vec<&i8> = self.entities.keys().collect();
        layers.sort();

        let draw_color = canvas.draw_color();

        // Iterate over sorted keys
        for layer in layers {
            if let Some(entities) = self.entities.get(layer) {
                for entity in entities.values() {
                    camera.draw(canvas, &entity.transform, 2, Vec3::new(255., 0., 0.))
                }
            }
        }

        canvas.set_draw_color(draw_color);
    }
}
