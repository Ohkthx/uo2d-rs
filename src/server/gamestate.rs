use std::collections::HashMap;
use std::thread::sleep;

use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::entity::Entity;
use crate::packet::payloads::MovementPayload;
use crate::packet::{Action, BroadcastScope, Packet, PacketConfiguration, Payload};
use crate::spatial_hash::SpatialHash;
use crate::sprintln;
use crate::timer::{TimerData, TimerManager};

use super::PacketCacheAsync;

/// Ensures the integrity of the game.
pub struct Gamestate {
    sender: Sender<PacketConfiguration>,
    timers: TimerManager,
    cache: PacketCacheAsync,
    spatial: SpatialHash,
    entities: HashMap<Uuid, Entity>,
    boundary: (u32, u32),
}

impl Gamestate {
    /// Create a new Gamestate.
    pub fn new(
        tx: Sender<PacketConfiguration>,
        cache: PacketCacheAsync,
        boundary: (u32, u32),
    ) -> Self {
        Self {
            sender: tx,
            timers: TimerManager::new(),
            cache,
            entities: HashMap::new(),
            spatial: SpatialHash::new(32),
            boundary,
        }
    }

    /// Obtains all pending packets from the cache.
    pub async fn get_packets(&mut self) -> Vec<Packet> {
        self.cache.get_all().await
    }

    /// Starts the servers gameloop.
    pub async fn start(&mut self) {
        // Create a test timer of 100 ticks and 5 seconds.
        self.timers.add_timer_tick(1000, TimerData::Empty);
        self.timers.add_timer_sec(5.0, TimerData::Empty, true);

        'running: loop {
            for timer in self.timers.update() {
                sprintln!("Expired: {:?}", timer);
            }

            self.update();

            // Process the data from the server if there is any.
            let packets = self.get_packets().await;
            for packet in packets.into_iter() {
                if packet.action() == Action::Shutdown {
                    break 'running;
                } else if packet.action() == Action::ClientLeave {
                    // Remove the entity from the world.
                    if let Some(entity) = self.entities.remove(&packet.uuid()) {
                        self.spatial.remove_entity(&entity)
                    }
                    continue;
                }

                if let Payload::Movement(movement) = packet.payload() {
                    self.movement(packet.uuid(), movement);
                }
            }
            sleep(self.timers.server_tick_time());
        }
    }

    fn movement(&mut self, uuid: Uuid, movement: MovementPayload) {
        self.entities
            .entry(uuid)
            .or_insert_with(|| Entity::new(uuid, movement.position, movement.size));

        let entity = match self.entities.get(&uuid) {
            None => return,
            Some(entity) => entity,
        };

        let mut query = entity.check_move(
            &mut self.spatial,
            self.boundary,
            movement.position,
            movement.trajectory,
            movement.speed,
        );

        let pos = match SpatialHash::till_collisions(&query, &self.entities) {
            Some(pos) => pos,
            None => return,
        };

        // Perform move.
        if let Some(entity) = self.entities.get_mut(&uuid) {
            query.destination = pos;
            entity.move_entity(&mut self.spatial, &query);
            if query.has_moved() {
                let _ = self.sender.try_send(PacketConfiguration::Broadcast(
                    Packet::new(
                        Action::Movement,
                        uuid,
                        Payload::Movement(MovementPayload::new(
                            movement.size,
                            entity.object().position(),
                            entity.last_trajectory,
                            movement.speed,
                        )),
                    ),
                    // Movement will only be sent to the nearby entities.
                    BroadcastScope::Local(query.nearby),
                ));
            }
        }
    }

    /// Called on every tick for the server.
    fn update(&mut self) {}
}
