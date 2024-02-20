use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::{Vec2, Vec3};

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

/// Movement payload, used to send current position for an entity.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MovementPayload {
    pub size: Vec2,
    pub position: Vec3,
    pub velocity: Vec2,
}

impl MovementPayload {
    /// Create a new position payload.
    pub fn new(size: Vec2, position: Vec3, velocity: Vec2) -> Self {
        Self {
            size,
            position,
            velocity,
        }
    }
}
