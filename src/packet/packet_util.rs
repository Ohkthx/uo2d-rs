use std::error::Error;
use uuid::Uuid;

use super::{Action, Payload, PACKET_VERSION};

/// Data that is used to communicate between clients and the server.
#[derive(Debug)]
pub struct Packet {
    /// Version of the packet.
    pub version: u8,
    /// Action being performed.
    pub action: Action,
    /// UUID of the client.
    pub uuid: Uuid,
    /// Data to be processed.
    pub payload: Payload,
}

impl Packet {
    /// Constructs a packet from the given components.
    pub fn new(action: Action, uuid: Uuid, payload: Payload) -> Packet {
        Packet {
            version: PACKET_VERSION,
            action,
            uuid,
            payload,
        }
    }

    /// Converts the packet into a byte array for sending.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut packet = Vec::new();

        // Packet Version
        packet.push(PACKET_VERSION);

        // Action (2 bytes)
        packet.extend_from_slice(&self.action.to_u16().to_be_bytes());

        // Player UUID (16 bytes)
        packet.extend_from_slice(self.uuid.as_bytes());

        // Serialized Payload
        packet.extend_from_slice(&bincode::serialize(&self.payload)?);

        Ok(packet)
    }

    /// Converts from a byte array for processing.
    pub fn from_bytes(packet: &[u8]) -> Result<Packet, Box<dyn Error>> {
        if packet.len() < 19 {
            return Err("Packet is too short".into());
        }

        let version = packet[0];
        let action = Action::from_bytes(&[packet[1], packet[2]]);
        let uuid = Uuid::from_slice(&packet[3..19])?;

        // Here you need to determine the type of Payload based on the action and deserialize accordingly
        let payload: Payload = bincode::deserialize(&packet[19..])?;

        Ok(Packet {
            version,
            action,
            uuid,
            payload,
        })
    }
}
