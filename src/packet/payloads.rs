use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::object::Position;

/// Message payload, only contains text.
#[derive(Serialize, Deserialize, Debug)]
pub struct MessagePayload {
    pub message: String,
}

impl MessagePayload {
    /// Create a new message payload.
    pub fn new(message: impl ToString) -> MessagePayload {
        MessagePayload {
            message: message.to_string(),
        }
    }
}

/// Ping payload, used to send current ping UUID.
#[derive(Serialize, Deserialize, Debug)]
pub struct PingPayload {
    pub uuid: Uuid,
}

impl PingPayload {
    /// Create a new ping payload.
    pub fn new(uuid: Uuid) -> PingPayload {
        PingPayload { uuid }
    }
}

/// Movement payload, used to send current position for an entity.
#[derive(Serialize, Deserialize, Debug)]
pub struct MovementPayload {
    pub size: (u16, u16),
    pub position: Position,
    pub trajectory: (f32, f32),
    pub speed: f32,
}

impl MovementPayload {
    /// Create a new position payload.
    pub fn new(
        size: (u16, u16),
        position: Position,
        trajectory: (f32, f32),
        speed: f32,
    ) -> MovementPayload {
        MovementPayload {
            size,
            position,
            trajectory,
            speed,
        }
    }
}
