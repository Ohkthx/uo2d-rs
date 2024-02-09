use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub position: (i32, i32),
}

impl MovementPayload {
    /// Create a new position payload.
    pub fn new(position: (i32, i32)) -> MovementPayload {
        MovementPayload { position }
    }
}
