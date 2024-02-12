use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

use uuid::Uuid;

use crate::{cprintln, object::Position, timer::TimerManager, util::exec_rainbow};

/// Represents players within the game.
#[derive(Default)]
pub struct Entity {
    pub uuid: Uuid,
    pub color: (u8, u8, u8),
    pub position: Position,
    pub size: (u16, u16),
}

impl PartialEq for Entity {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Eq for Entity {}

impl Hash for Entity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

impl Entity {
    pub fn center_offset(&self, window_size: (u32, u32)) -> (i32, i32) {
        let (n, m) = (window_size.0 as i32, window_size.1 as i32);
        let (x, y, _z) = self.position;

        // Calculate the offsets to center (x, y) on (n, m)
        let a = x - n / 2;
        let b = y - m / 2;

        (a, b)
    }

    pub fn draw(&self, canvas: &mut WindowCanvas, offset: (i32, i32)) {
        let pos = (self.position.0 - offset.0, self.position.1 - offset.1);
        // Draw the border
        let border_rect = Rect::new(pos.0, pos.1, self.size.0 as u32, self.size.1 as u32);
        canvas.set_draw_color(Color::RGB(0, 0, 0)); // Black color for the border
        if let Err(why) = canvas.fill_rect(border_rect) {
            cprintln!("Unable to render border for {}: {}", self.uuid, why);
        }

        // Draw the base square on top of the border
        let base_rect = Rect::new(
            pos.0 + 2,
            pos.1 + 2,
            self.size.0 as u32 - 4,
            self.size.1 as u32 - 4,
        );
        canvas.set_draw_color(Color::RGB(self.color.0, self.color.1, self.color.2));
        if let Err(why) = canvas.fill_rect(base_rect) {
            cprintln!("Unable to render base for {}: {}", self.uuid, why);
        }
    }
}

/// Current tracked state of the game.
pub struct Gamestate {
    pub timers: TimerManager,
    locations: HashMap<Uuid, i8>,
    pub entities: HashMap<i8, HashMap<Uuid, Entity>>,
    next_color: (u8, u8, u8),
    pub kill: bool,
}

impl Gamestate {
    /// Initializes the gamestate.
    pub fn new() -> Self {
        Self {
            timers: TimerManager::new(),
            locations: HashMap::new(),
            entities: HashMap::new(),
            next_color: (0, 0, 0),
            kill: false,
        }
    }

    // Get an entity by its UUID
    pub fn get_entity(&self, uuid: &Uuid) -> Option<&Entity> {
        if let Some(layer) = self.locations.get(uuid) {
            if let Some(entities) = self.entities.get(layer) {
                return entities.get(uuid);
            }
        }
        None
    }

    /// Updates an entity's position and size, if it exists, or inserts a new entity.
    pub fn upsert_entity(&mut self, uuid: Uuid, position: Position, size: (u16, u16)) {
        let new_layer = position.2;
        let entity_color;

        if let Some(&old_layer) = self.locations.get(&uuid) {
            if old_layer != new_layer {
                // Entity has changed layers, remove it from the old layer and preserve its color.
                if let Some(entity) = self
                    .entities
                    .get_mut(&old_layer)
                    .and_then(|map| map.remove(&uuid))
                {
                    // Preserve the color of the moving entity.
                    entity_color = entity.color;
                } else {
                    // If not found or new entity, assign next color.
                    entity_color = self.next_color;
                    self.next_color = exec_rainbow(self.next_color, 35);
                }
            } else {
                // If the entity is in the same layer, use its existing color, no need to remove.
                entity_color = self
                    .entities
                    .get(&old_layer)
                    .and_then(|map| map.get(&uuid).map(|e| e.color))
                    .unwrap_or_else(|| {
                        self.next_color = exec_rainbow(self.next_color, 35);
                        self.next_color
                    });
            }
        } else {
            // New entity, assign next color.
            entity_color = self.next_color;
            self.next_color = exec_rainbow(self.next_color, 35);
        }

        // Create or update the entity with its new or existing color.
        let entity = Entity {
            uuid,
            color: entity_color,
            position,
            size,
        };

        // Assign entity to the new layer and update locations mapping.
        self.locations.insert(uuid, new_layer);
        self.entities
            .entry(new_layer)
            .or_default()
            .insert(uuid, entity);
    }

    pub fn remove_entity(&mut self, uuid: Uuid) {
        // First, find the layer the entity is in using the locations map and remove the entry.
        if let Some(layer) = self.locations.remove(&uuid) {
            // Then, access the sub-map for the layer and attempt to remove the entity by its UUID.
            if let Some(entities) = self.entities.get_mut(&layer) {
                entities.remove(&uuid);

                // Remove the layer if it is empty.
                if entities.is_empty() {
                    self.entities.remove(&layer);
                }
            }
        }
    }

    /// Draws all currently stored entities.
    pub fn draw(&self, canvas: &mut WindowCanvas, offset: (i32, i32)) {
        let mut layers: Vec<&i8> = self.entities.keys().collect();
        layers.sort();

        let draw_color = canvas.draw_color();

        // Iterate over sorted keys
        for layer in layers {
            if let Some(entities) = self.entities.get(layer) {
                for entity in entities.values() {
                    entity.draw(canvas, offset);
                }
            }
        }

        canvas.set_draw_color(draw_color);
    }
}
