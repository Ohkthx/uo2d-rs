mod packet_util;
pub mod payloads;

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
pub use packet_util::*;
use serde::{Deserialize, Serialize};

use self::payloads::*;

pub const PACKET_VERSION: u8 = 0x01;

/// Action that represents the Packet.
#[derive(Debug, FromPrimitive, ToPrimitive)]
pub enum Action {
    Ping = 0x1,
    Success,
    Error,
    Shutdown,
    ClientJoin,
    ClientLeave,
    Message,
}

impl Action {
    /// Convert the action from bytes.
    pub fn from_bytes(bytes: &[u8; 2]) -> Action {
        let value = u16::from_be_bytes([bytes[0], bytes[1]]);
        FromPrimitive::from_u16(value)
            .unwrap_or_else(|| panic!("Unable to convert Packet Action {} to Action.", value))
    }

    /// Convert to a numeric value.
    pub fn to_u16(&self) -> u16 {
        ToPrimitive::to_u16(self)
            .unwrap_or_else(|| panic!("Unable to convert Packet Action {:?} to u16.", self))
    }
}

/// Payloads that can be sent inside a packet.
#[derive(Serialize, Deserialize, Debug)]
pub enum Payload {
    Empty,
    Invalid,
    Ping(PingPayload),
    Message(MessagePayload),
}
