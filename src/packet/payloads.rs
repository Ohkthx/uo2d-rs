use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::{Vec2, Vec3};
use crate::ecs::Entity;

/// Message payload, only contains text.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessagePayload {
    pub message: String,
}

impl MessagePayload {
    /// Create a new message payload.
    pub fn new(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

/// Ping payload, used to send current ping UUID.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UuidPayload {
    pub uuid: Uuid,
}

impl UuidPayload {
    /// Create a new ping payload.
    pub fn new(uuid: Uuid) -> Self {
        Self { uuid }
    }
}

/// Entity payload, used to send an Entity.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EntityPayload {
    pub entity: Entity,
}

impl EntityPayload {
    /// Create a new entity payload.
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

/// Movement payload, used to send current position for an entity.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MovementPayload {
    pub entity: Entity,
    pub size: Vec2,
    pub position: Vec3,
    pub velocity: Vec2,
}

impl MovementPayload {
    /// Create a new position payload.
    pub fn new(entity: Entity, size: Vec2, position: Vec3, velocity: Vec2) -> Self {
        Self {
            entity,
            size,
            position,
            velocity,
        }
    }
}
