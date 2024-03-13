use std::collections::{HashMap, HashSet};
use std::thread::sleep;

use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::components::{Bounds, Player, Position, Projectile, Vec2, Vec3, Velocity};
use crate::ecs::{Entity, World};
use crate::entities::{Region, RegionManager};
use crate::packet::payloads::{EntityPayload, MovementPayload};
use crate::packet::{Action, BroadcastScope, Packet, PacketConfiguration, Payload};
use crate::spatial_hash::SpatialHash;
use crate::sprintln;
use crate::timer::{TimerData, TimerManager};

use super::systems::movement::{self};
use super::{systems, PacketCacheAsync};

/// Ensures the integrity of the game.
pub struct Gamestate {
    world: World,
    sender: Sender<PacketConfiguration>,
    timers: TimerManager,
    cache: PacketCacheAsync,
    spatial: SpatialHash,
    regions: RegionManager,
    players: HashMap<Uuid, Entity>,
}

impl Gamestate {
    const PROJECTILE_LIFESPAN: f32 = 10.0;

    /// Create a new Gamestate.
    pub fn new(tx: Sender<PacketConfiguration>, cache: PacketCacheAsync) -> Self {
        let regions = RegionManager::new();

        // Create the world and register the components.
        let mut world = World::new();
        world.register_component::<Position>();
        world.register_component::<Velocity>();
        world.register_component::<Player>();
        world.register_component::<Projectile>();

        Self {
            world,
            sender: tx,
            timers: TimerManager::new(),
            cache,
            spatial: SpatialHash::new(32),
            regions,
            players: HashMap::new(),
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

    /// Obtains a player based on its UUID.
    pub fn get_player(&self, uuid: &Uuid) -> Option<(Entity, Player)> {
        if let Some(entity) = self.players.get(uuid) {
            if let Some(player) = self.world.get_component::<Player>(entity) {
                return Some((*entity, *player));
            }
        }

        None
    }

    /// Remove a player.
    fn remove_player(&mut self, uuid: &Uuid) -> Option<(Entity, Player)> {
        if let Some((entity, player)) = self.get_player(uuid) {
            if let Some(pos) = self.world.get_component::<Position>(&entity) {
                // Remove space it is taking up.
                let bounds = Bounds::from_vec(pos.loc, pos.size);
                self.spatial.remove_object(&entity, &bounds)
            }

            // Remove / despawn the entity from the ECS.
            self.world.despawn(&entity);
            return Some((entity, player));
        }

        None
    }

    /// Add a new player.
    fn add_player(&mut self, uuid: Uuid) -> (Entity, Player, Position) {
        let position = Position::new(self.get_spawn_region().spawn, Vec2::new(32., 32.));
        let player = Player::new(uuid);

        // Add player to the world and gamestate for tracking.
        let entity = self.world.spawn().with(position).with(player).build();
        self.players.insert(*player.uuid(), entity);

        (entity, player, position)
    }

    /// Obtain all nearby players.
    fn get_nearby(&self, player: &Entity, range: f64) -> Vec<(Entity, Player)> {
        movement::get_nearby(&self.world, &self.spatial, player, range)
    }

    /// Starts the servers gameloop.
    pub async fn start(&mut self) {
        // Create a test timer of 100 ticks and 5 seconds.
        self.timers.add_timer_tick(1000, TimerData::Empty);
        self.timers.add_timer_sec(5.0, TimerData::Empty, true);

        'running: loop {
            for timer in self.timers.update() {
                if let TimerData::EntityDelete(entity) = timer.data {
                    let nearby: HashSet<Uuid> = self
                        .get_nearby(&entity, 10.)
                        .iter()
                        .map(|(_e, p)| *p.uuid())
                        .collect();

                    self.world.despawn(&entity);

                    // Send a packet to nearby players that it has been despawned.
                    let _ = self.sender.try_send(PacketConfiguration::Broadcast(
                        Packet::new(
                            Action::EntityDelete,
                            Uuid::nil(),
                            Payload::Entity(EntityPayload::new(entity)),
                        ),
                        BroadcastScope::Local(nearby),
                    ));
                }
            }

            // Process the data from the clients if there is any.
            let packets = self.get_packets().await;
            for packet in packets.into_iter() {
                let uuid = packet.uuid();
                match packet.action() {
                    Action::Shutdown => break 'running,
                    Action::ClientJoin => self.join(uuid),
                    Action::ClientLeave => self.leave(&uuid),
                    Action::Movement => self.movement(uuid, packet.payload()),
                    Action::Projectile => self.projectile(packet.payload()),
                    _ => (),
                };
            }

            self.update();
            sleep(
                self.timers
                    .server_tick_time()
                    .saturating_sub(self.timers.tick_time()),
            );
        }
    }

    fn join(&mut self, uuid: Uuid) {
        let (entity, _player, position) = self.add_player(uuid);
        sprintln!("Player [{}] {} joined.", entity, uuid);

        let payload = Payload::Movement(MovementPayload::new(
            entity,
            position.size,
            position.loc,
            Vec2::ORIGIN,
        ));

        let nearby = self
            .get_nearby(&entity, 10.)
            .into_iter()
            .map(|(_e, p)| *p.uuid())
            .collect();

        let _ = self.sender.try_send(PacketConfiguration::SuccessBroadcast(
            Packet::new(Action::Success, uuid, payload.clone()),
            Packet::new(Action::ClientJoin, uuid, payload),
            BroadcastScope::Local(nearby),
        ));
    }

    fn leave(&mut self, uuid: &Uuid) {
        if let Some((entity, _player)) = self.remove_player(uuid) {
            sprintln!("Player [{}] {} left.", entity, uuid);

            let _ = self.sender.try_send(PacketConfiguration::Broadcast(
                Packet::new(
                    Action::ClientLeave,
                    *uuid,
                    Payload::Entity(EntityPayload::new(entity)),
                ),
                BroadcastScope::Global,
            ));
        }
    }

    fn movement(&mut self, uuid: Uuid, movement: Payload) {
        let movement = match movement {
            Payload::Movement(movement) => movement,
            _ => return,
        };

        if let Some((entity, _player)) = self.get_player(&uuid) {
            self.world
                .upsert_component(entity, Velocity(movement.velocity));
        }
    }

    fn projectile(&mut self, payload: Payload) {
        let movement = match payload {
            Payload::Movement(movement) => movement,
            _ => return,
        };

        let position = Position::new(movement.position, movement.size);
        let entity = self
            .world
            .spawn()
            .with(position)
            .with(Velocity(movement.velocity))
            .with(Projectile {})
            .build();

        // Projectiles have timed life.
        self.timers.add_timer_sec(
            Self::PROJECTILE_LIFESPAN,
            TimerData::EntityDelete(entity),
            true,
        );
    }

    /// Called on every tick for the server.
    fn update(&mut self) {
        let mut packets: Vec<PacketConfiguration> = vec![];
        packets.extend(systems::movement::with_velocity(
            &mut self.world,
            &mut self.spatial,
            &self.regions,
        ));

        for packet in packets.into_iter() {
            let _ = self.sender.try_send(packet);
        }
    }
}
