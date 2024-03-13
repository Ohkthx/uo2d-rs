use std::collections::HashMap;

use sdl2::render::WindowCanvas;

use crate::components::{Vec2, Vec3};
use crate::ecs::Entity;
use crate::entities::{Camera, Mobile};
use crate::timer::TimerManager;

/// Current tracked state of the game.
pub struct Gamestate {
    pub timers: TimerManager,
    locations: HashMap<Entity, i8>,
    pub entities: HashMap<i8, HashMap<Entity, Mobile>>,
    pub kill: bool,
    player: Entity,
}

impl Gamestate {
    /// Initializes the gamestate.
    pub fn new() -> Self {
        Self {
            timers: TimerManager::new(),
            locations: HashMap::new(),
            entities: HashMap::new(),
            kill: false,
            player: Entity::INVALID,
        }
    }

    /// Sets the player / entity belonging to the client.
    pub fn set_player(&mut self, entity: Entity) {
        self.player = entity;
    }

    /// Obtains the entity representing the client.
    pub fn get_player(&self) -> Entity {
        self.player
    }

    // Get a mobile by its entity.
    pub fn get_mobile(&self, entity: &Entity) -> Option<&Mobile> {
        if let Some(layer) = self.locations.get(entity) {
            if let Some(entities) = self.entities.get(layer) {
                return entities.get(entity);
            }
        }
        None
    }

    /// Updates an entity's position and size, if it exists, or inserts a new entity.
    pub fn upsert_entity(&mut self, entity: Entity, position: Vec3, size: Vec2) {
        // Create or update the entity with its new or existing color.
        let mobile = Mobile::new(entity, position, size);

        // Assign entity to the new layer and update locations mapping.
        self.locations.insert(entity, position.z() as i8);
        self.entities
            .entry(position.z() as i8)
            .or_default()
            .insert(entity, mobile);
    }

    /// Removes an entity from being tracked.
    pub fn remove_entity(&mut self, entity: &Entity) {
        // First, find the layer the entity is in using the locations map and remove the entry.
        if let Some(layer) = self.locations.remove(entity) {
            // Then, access the sub-map for the layer and attempt to remove the entity by its UUID.
            if let Some(entities) = self.entities.get_mut(&layer) {
                entities.remove(entity);

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
