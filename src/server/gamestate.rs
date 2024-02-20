use std::collections::HashMap;
use std::thread::sleep;

use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::components::{Bounds, Vec2, Vec3};
use crate::entities::{Mobile, MobileType, Region, RegionManager};
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
    entities: HashMap<Uuid, Mobile>,
    regions: RegionManager,
}

impl Gamestate {
    const PROJECTILE_LIFESPAN: f32 = 10.0;
    const MAX_SPEED: f64 = 5.;

    /// Create a new Gamestate.
    pub fn new(tx: Sender<PacketConfiguration>, cache: PacketCacheAsync) -> Self {
        let regions = RegionManager::new();

        Self {
            sender: tx,
            timers: TimerManager::new(),
            cache,
            entities: HashMap::new(),
            spatial: SpatialHash::new(32),
            regions,
        }
    }

    /// Obtains all pending packets from the cache.
    pub async fn get_packets(&mut self) -> Vec<Packet> {
        self.cache.get_all().await
    }

    /// Get the spawn location.
    pub fn get_spawn_region(&self) -> &Region {
        self.regions
            .get_region(&Vec3::new(512., 512., 1.))
            .expect("Spawn region is not set!")
    }

    /// Attempts to reverse lookup region from coordinates.
    pub fn get_region(&self, position: &Vec3) -> Option<&Region> {
        self.regions.get_region(position)
    }

    /// Remove an entity.
    fn remove_entity(&mut self, uuid: &Uuid) {
        if let Some(entity) = self.entities.remove(uuid) {
            self.spatial
                .remove_object(&entity.uuid, &entity.transform.bounding_box())
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
                        let range = entity.transform.bounding_box().scaled_center(10.);
                        let nearby = self.spatial.query(&range, Some(&uuid));
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
                    Action::Movement => self.movement(uuid, packet.payload()),
                    Action::Projectile => self.projectile(uuid, packet.payload()),
                    _ => (),
                };
            }
            sleep(self.timers.server_tick_time());
        }
    }

    fn join(&mut self, uuid: Uuid) {
        let position: Vec3 = self.get_spawn_region().spawn;
        sprintln!("Spawn set to: {:?}", position);
        let size = Vec2::new(32., 32.);

        self.entities
            .entry(uuid)
            .or_insert_with(|| Mobile::new(uuid, position, size, MobileType::Creature));

        let entity = match self.entities.get(&uuid) {
            None => return,
            Some(entity) => entity,
        };
        self.spatial.insert_object(&uuid, &entity.bounding_box());

        let payload = Payload::Movement(MovementPayload::new(
            entity.bounding_box().dimensions(),
            entity.bounding_box().top_left_3d(),
            Vec2::ORIGIN,
        ));

        // The scope of who to send these packets to.
        let range = entity.bounding_box().scaled_center(10.);
        let nearby = self.spatial.query(&range, Some(&uuid));
        let scope = BroadcastScope::Local(nearby);

        let _ = self.sender.try_send(PacketConfiguration::SuccessBroadcast(
            Packet::new(Action::Success, uuid, payload.clone()),
            Packet::new(Action::ClientJoin, uuid, payload),
            scope,
        ));
    }

    fn movement(&mut self, uuid: Uuid, movement: Payload) {
        let movement = match movement {
            Payload::Movement(movement) => movement,
            _ => return,
        };

        // Only move valid existing entities.
        let entity = match self.entities.get(&uuid) {
            None => return,
            Some(entity) => entity,
        };

        // Try to locate the region.
        let region = match self.get_region(&entity.transform.position()) {
            Some(region) => region.clone(),
            None => {
                sprintln!("Unable to find region for {}", entity.uuid);
                return;
            }
        };

        // Get the attempted movement.
        let velocity = movement.velocity.clamped(0.0, Self::MAX_SPEED);
        let mut query = entity.check_move(&mut self.spatial, &region, velocity);

        let pos = match SpatialHash::till_collisions(&query, &self.entities) {
            Some(pos) => pos,
            None => {
                // Unavoidable collision detected.
                query.source
            }
        };

        // Perform move.
        if let Some(mobile) = self.entities.get_mut(&uuid) {
            query.destination = pos;
            mobile.move_entity(&mut self.spatial, &query);
            if query.has_moved() {
                let _ = self.sender.try_send(PacketConfiguration::Broadcast(
                    Packet::new(
                        Action::Movement,
                        uuid,
                        Payload::Movement(MovementPayload::new(
                            movement.size,
                            mobile.transform.position(),
                            movement.velocity,
                        )),
                    ),
                    // Movement will only be sent to the nearby entities.
                    BroadcastScope::Local(query.nearby),
                ));
            }
        }
    }

    fn projectile(&mut self, uuid: Uuid, payload: Payload) {
        let movement = match payload.clone() {
            Payload::Movement(movement) => movement,
            _ => return,
        };

        // Try to locate the region and only spawn if it is within region bounds.
        match self.get_region(&movement.position) {
            Some(region) => {
                let bounds = Bounds::from_vec(movement.position, movement.size);
                if !region.is_inbounds(&bounds) {
                    return;
                }
            }
            None => return,
        };

        self.entities.entry(uuid).or_insert_with(|| {
            Mobile::new(
                uuid,
                movement.position,
                movement.size,
                MobileType::Projectile,
            )
        });

        let entity = match self.entities.get(&uuid) {
            None => return,
            Some(entity) => entity,
        };
        self.spatial
            .insert_object(&entity.uuid, &entity.bounding_box());

        self.timers.add_timer_sec(
            Self::PROJECTILE_LIFESPAN,
            TimerData::EntityDelete(uuid),
            true,
        );

        self.movement(uuid, payload);
    }

    /// Called on every tick for the server.
    fn update(&mut self) {
        let entities: Vec<Mobile> = self
            .entities
            .values()
            .filter(|e| e.mobile_type == MobileType::Projectile)
            .cloned()
            .collect();

        for entity in entities {
            // Move autonomous entities.
            if entity.has_moved && entity.last_position != entity.position() {
                self.movement(
                    entity.uuid,
                    Payload::Movement(MovementPayload::new(
                        entity.bounding_box().dimensions(),
                        entity.bounding_box().top_left_3d(),
                        entity.last_velocity,
                    )),
                )
            } else {
                // If it is stuck, delete it.
                let range = entity.bounding_box().scaled_center(10.);
                let nearby = self.spatial.query(&range, Some(&entity.uuid));
                let _ = self.sender.try_send(PacketConfiguration::Broadcast(
                    Packet::new(Action::EntityDelete, entity.uuid, Payload::Empty),
                    BroadcastScope::Local(nearby),
                ));
                self.remove_entity(&entity.uuid);
            }
        }
    }
}
