use std::{thread::sleep, time::Duration};

use tokio::sync::mpsc::Sender;

use crate::packet::{Action, BroadcastScope, Packet, PacketConfiguration, Payload};

use super::PacketCache;

/// Ensures the integrity of the game.
pub struct Gamestate {
    sender: Sender<PacketConfiguration>,
    cache: PacketCache,
}

impl Gamestate {
    /// Create a new Gamestate.
    pub fn new(tx: Sender<PacketConfiguration>, cache: PacketCache) -> Self {
        Self { sender: tx, cache }
    }

    /// Obtains all pending packets from the cache.
    pub fn get_packets(&mut self) -> Vec<Packet> {
        self.cache.get_all()
    }

    /// Starts the servers gameloop.
    pub fn start(&mut self) {
        'running: loop {
            // Process the data from the server if there is any.
            let packets = self.get_packets();
            for packet in packets.into_iter() {
                if packet.action() == Action::Shutdown {
                    break 'running;
                }

                let _ = match packet.payload() {
                    Payload::Movement(_) => self.sender.try_send(PacketConfiguration::Broadcast(
                        packet,
                        BroadcastScope::Local,
                    )),
                    _ => Ok(()),
                };
            }
            sleep(Duration::from_millis(16));
        }
    }
}
