use std::collections::HashMap;
use std::thread::sleep;

use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::entity::{Entity, EntityType};
use crate::object::Position;
use crate::packet::payloads::MovementPayload;
use crate::packet::{Action, BroadcastScope, Packet, PacketConfiguration, Payload};
use crate::spatial_hash::SpatialHash;
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
    const PROJECTILE_LIFESPAN: f32 = 10.0;

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

    /// Remove an entity.
    fn remove_entity(&mut self, uuid: &Uuid) {
        if let Some(entity) = self.entities.remove(uuid) {
            self.spatial.remove_entity(&entity)
        }
    }

    /// Starts the servers gameloop.
    pub async fn start(&mut self) {
        // Create a test timer of 100 ticks and 5 seconds.
        self.timers.add_timer_tick(1000, TimerData::Empty);
        self.timers.add_timer_sec(5.0, TimerData::Empty, true);

        'running: loop {
            for timer in self.timers.update() {
                if let TimerData::EntityDelete(uuid) = timer.data {
                    if let Some(entity) = self.entities.get(&uuid) {
                        let nearby = self.spatial.query(&entity.object().range(10), Some(uuid));
                        let _ = self.sender.try_send(PacketConfiguration::Broadcast(
                            Packet::new(Action::EntityDelete, uuid, Payload::Empty),
                            BroadcastScope::Local(nearby),
                        ));
                        self.remove_entity(&uuid);
                    }
                }
            }

            self.update();

            // Process the data from the server if there is any.
            let packets = self.get_packets().await;
            for packet in packets.into_iter() {
                let uuid = packet.uuid();
                match packet.action() {
                    Action::Shutdown => break 'running,
                    Action::ClientJoin => self.join(uuid),
                    Action::ClientLeave => self.remove_entity(&uuid),
                    Action::Movement => self.movement(uuid, packet.payload(), false),
                    Action::Projectile => self.movement(uuid, packet.payload(), true),
                    _ => (),
                };
            }
            sleep(self.timers.server_tick_time());
        }
    }

    fn join(&mut self, uuid: Uuid) {
        let position: Position = (self.boundary.0 as i32 / 2, self.boundary.1 as i32 / 2, 1);
        let size = (32, 32);

        self.entities
            .entry(uuid)
            .or_insert_with(|| Entity::new(uuid, position, size, EntityType::Creature));

        let entity = match self.entities.get(&uuid) {
            None => return,
            Some(entity) => entity,
        };
        self.spatial.insert_entity(entity);

        let payload = Payload::Movement(MovementPayload::new(
            entity.object().size(),
            entity.object().position(),
            (0.0, 0.0),
            0.0,
        ));

        // The scope of who to send these packets to.
        let nearby = self.spatial.query(&entity.object().range(10), Some(uuid));
        let scope = BroadcastScope::Local(nearby);

        let _ = self.sender.try_send(PacketConfiguration::SuccessBroadcast(
            Packet::new(Action::Success, uuid, payload.clone()),
            Packet::new(Action::ClientJoin, uuid, payload),
            scope,
        ));
    }

    fn movement(&mut self, uuid: Uuid, movement: Payload, is_projectile: bool) {
        let movement = match movement {
            Payload::Movement(movement) => movement,
            _ => return,
        };

        // Give a limited lifespan to a projectile.
        let entity_type = if is_projectile {
            self.timers.add_timer_sec(
                Self::PROJECTILE_LIFESPAN,
                TimerData::EntityDelete(uuid),
                true,
            );
            EntityType::Projectile
        } else {
            EntityType::Creature
        };

        self.entities
            .entry(uuid)
            .or_insert_with(|| Entity::new(uuid, movement.position, movement.size, entity_type));

        let entity = match self.entities.get(&uuid) {
            None => return,
            Some(entity) => entity,
        };

        let mut query = entity.check_move(
            &mut self.spatial,
            self.boundary,
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
                            movement.trajectory,
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
    fn update(&mut self) {
        let entities: Vec<Entity> = self
            .entities
            .values()
            .filter(|e| e.entity_type == EntityType::Projectile)
            .cloned()
            .collect();

        for entity in entities {
            // Move autonomous entities.
            if entity.has_moved {
                self.movement(
                    entity.uuid,
                    Payload::Movement(MovementPayload::new(
                        entity.object().size(),
                        entity.object().position(),
                        entity.last_trajectory,
                        5.0,
                    )),
                    true,
                )
            } else {
                // If it is stuck, delete it.
                let nearby = self
                    .spatial
                    .query(&entity.object().range(10), Some(entity.uuid));
                let _ = self.sender.try_send(PacketConfiguration::Broadcast(
                    Packet::new(Action::EntityDelete, entity.uuid, Payload::Empty),
                    BroadcastScope::Local(nearby),
                ));
                self.remove_entity(&entity.uuid);
            }
        }
    }
}
