use uuid::Uuid;

use crate::sprintln;

use super::{Action, Payload, PACKET_VERSION};

const DATA_BASE_SIZE: usize = 32;

/// Represents data being sent between server and clients.
#[derive(Debug, Clone)]
pub struct Packet {
    data: Vec<u8>,
}

impl Packet {
    /// Constructs a packet from the given components, and serializes them into the internal data vector.
    pub fn new(action: Action, uuid: Uuid, payload: Payload) -> Packet {
        let packet = Self {
            data: vec![0u8; DATA_BASE_SIZE],
        };

        // Packet Version
        packet
            .set_version(PACKET_VERSION)
            .set_action(action)
            .set_uuid(uuid)
            .set_payload(payload)
    }

    /// Returns the packet version.
    #[allow(dead_code)]
    pub fn version(&self) -> u8 {
        self.data[0]
    }

    /// Returns the packet action.
    pub fn action(&self) -> Action {
        let action_bytes = [self.data[1], self.data[2]];
        Action::from_bytes(&action_bytes)
    }

    /// Returns the packet UUID.
    pub fn uuid(&self) -> Uuid {
        Uuid::from_slice(&self.data[3..19]).unwrap()
    }

    /// Returns the packet payload, deserialized.
    pub fn payload(&self) -> Payload {
        match bincode::deserialize(&self.data[19..]) {
            Ok(payload) => payload,
            Err(_) => {
                sprintln!("Got a bad payload from {}.", self.uuid());
                Payload::Invalid
            }
        }
    }

    /// Sets the version in the packet.
    pub fn set_version(mut self, version: u8) -> Self {
        self.data[0] = version;
        self
    }

    /// Sets the action in the packet.
    pub fn set_action(mut self, action: Action) -> Self {
        let action_bytes = action.to_u16().to_be_bytes();
        self.data[1..3].copy_from_slice(&action_bytes);
        self
    }

    /// Sets the UUID in the packet.
    pub fn set_uuid(mut self, uuid: Uuid) -> Self {
        self.data[3..19].copy_from_slice(uuid.as_bytes());
        self
    }

    /// Sets the payload in the packet. This method assumes the payload starts at byte 19.
    /// It resizes the data vector if the serialized payload is larger than the initial allocation.
    pub fn set_payload(mut self, payload: Payload) -> Self {
        let payload_bytes =
            bincode::serialize(&payload).expect("unable to serialize the payload for a packet");
        if payload_bytes.len() > self.data.len() - 19 {
            // Extend the data vector to fit the new payload, only if necessary
            self.data.resize(19 + payload_bytes.len(), 0);
        }
        self.data[19..19 + payload_bytes.len()].copy_from_slice(&payload_bytes);
        self
    }

    /// Converts the packet into a byte array for sending.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Converts from a byte array to a Packet, resizing the byte array if it is not at least 20 bytes long.
    pub fn from_bytes(bytes: &[u8]) -> Packet {
        let mut data: Vec<u8> = vec![0; DATA_BASE_SIZE.max(bytes.len())];
        data[..bytes.len()].copy_from_slice(bytes);

        Packet { data }
    }
}
