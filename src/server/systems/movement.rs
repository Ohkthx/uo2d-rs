use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::components::{Bounds, Player, Position, Projectile, Transform, Vec2, Vec3, Velocity};
use crate::ecs::{ComponentChange, Entity, World};
use crate::entities::{Region, RegionManager};
use crate::packet::payloads::{EntityPayload, MovementPayload};
use crate::packet::{Action, BroadcastScope, Packet, PacketConfiguration, Payload};
use crate::spatial_hash::SpatialHash;

/// A query to move an entity. Useful to check multiple movements in 1 tick.
#[derive(Debug)]
pub struct MoveQuery {
    pub entity: Entity,
    pub source: Vec3,
    pub destination: Vec3,
    pub velocity: Vec2,
    pub entity_size: Vec2,
    pub nearby: HashSet<Entity>,
}

impl MoveQuery {
    /// Checks if a move has happened.
    pub fn has_moved(&self) -> bool {
        self.source.round() != self.destination.round()
    }

    /// Checks if the entitiy is potentialy stuck against a boundary or collision.
    pub fn is_stuck(&self) -> bool {
        !self.has_moved() && self.velocity != Vec2::ORIGIN
    }

    // The bounds for the query based on the position provided.
    pub fn bounds(&self, pos: Vec3) -> Bounds {
        Bounds::from_vec(pos, self.entity_size)
    }
}

/// A system used to process all entities that have positions and velocities. Essentially this is currently moving entities.
pub fn with_velocity(
    world: &mut World,
    spatial: &mut SpatialHash,
    regions: &RegionManager,
) -> Vec<PacketConfiguration> {
    let mut pos_changes: Vec<ComponentChange<Position>> = vec![];
    let mut vel_changes: Vec<ComponentChange<Velocity>> = vec![];
    let mut despawn: Vec<Entity> = vec![];

    let mut packets = vec![];
    let positions: HashMap<Entity, &Position> = world.query1::<Position>().into_iter().collect();

    // Iterate all entities with position and velocity.
    for (entity, pos, vel) in world.query2::<Position, Velocity>() {
        // Obtain the region for the entity.
        let region = match regions.get_region(&pos.loc) {
            Some(r) => r,
            None => continue,
        };

        // Limit the velocity to the maximum speed.
        let is_projectile = world.get_component::<Projectile>(&entity).is_some();
        let mut step = 1.0;
        let velocity = if is_projectile {
            vel.0.clamped(0., region.tile_length())
        } else {
            step = region.tile;
            let tile_size = region.tile_size();
            vel.0.clamp(tile_size.scaled(-1.), tile_size)
        };

        // Get the movement query and check if it can move.
        let mut query = check_move(spatial, region, entity, *pos, velocity, !is_projectile);
        let pos = match SpatialHash::till_collisions(&query, &positions, step) {
            Some(pos) => pos,
            None => {
                // Unavoidable collision detected.
                query.source
            }
        };

        // Obtains the nearby players.
        let nearby = get_nearby(world, spatial, &entity, 10.)
            .into_iter()
            .map(|(_e, p)| *p.uuid())
            .collect();

        // Did not move. Remove velocity.
        let has_passed = query.velocity.length() > vel.0.length();
        if pos == query.source || query.is_stuck() || has_passed {
            if is_projectile {
                // It is a projectile that cannot move, delete it.
                despawn.push(entity);
                spatial.remove_object(&query.entity, &query.bounds(query.source));
                packets.push(PacketConfiguration::Broadcast(
                    Packet::new(
                        Action::EntityDelete,
                        Uuid::nil(),
                        Payload::Entity(EntityPayload::new(entity)),
                    ),
                    BroadcastScope::Local(nearby),
                ));
                continue;
            }

            vel_changes.push(ComponentChange::Remove(entity));
            if !has_passed {
                continue;
            }
        }

        // Entity moved, position and velocity need to be updated.
        query.destination = pos;
        let pos_change = Position::new(query.destination, query.entity_size);
        let vel_change = vel.0.offset_from(&query.velocity);
        pos_changes.push(ComponentChange::Update(entity, pos_change));
        vel_changes.push(ComponentChange::Update(entity, Velocity(vel_change)));
        move_entity(spatial, &query);

        // Set the packet to be sent.
        packets.push(PacketConfiguration::Broadcast(
            Packet::new(
                Action::Movement,
                Uuid::nil(),
                Payload::Movement(MovementPayload::new(
                    entity,
                    query.entity_size,
                    query.destination,
                    query.velocity,
                )),
            ),
            // Movement will only be sent to the nearby entities.
            BroadcastScope::Local(nearby),
        ));
    }

    // Process the updates / component changes.
    ComponentChange::<Velocity>::processor(world, vel_changes);
    ComponentChange::<Position>::processor(world, pos_changes);

    // Despawn all entities flagged.
    for entity in despawn.into_iter() {
        world.despawn(&entity);
    }

    packets
}

/// Checks the entities attempted movement to ensure it is within the boundaries. Returns a MoveQuery used to check collision with other entities.
fn check_move(
    spatial: &mut SpatialHash,
    region: &Region,
    entity: Entity,
    position: Position,
    velocity: Vec2,
    align: bool,
) -> MoveQuery {
    // Apply movement deltas within bounds.
    let mut transform = Transform::from_bounds(position.bounds());
    transform = transform.applied_velocity(&velocity, &region.bounding_box(), align);

    // Align to the regions tile size.
    let velocity = if align {
        let alignment = region.align_coord(transform.position());
        transform.set_position(&alignment);
        transform.position().offset_from_2d(&position.loc).as_vec2()
    } else {
        velocity
    };

    // Builds the query.
    let mut query = MoveQuery {
        entity,
        source: position.loc,
        destination: transform.position(),
        velocity,
        entity_size: position.size,
        nearby: HashSet::new(),
    };

    // Get nearby entities.
    if query.has_moved() {
        let bounds = transform.bounding_box();
        query.nearby = spatial.query(&bounds, Some(&entity));
    }

    query
}

/// Finalizes the movement utilizing the query. Updates the spatial hash with the new position.
pub fn move_entity(spatial_area: &mut SpatialHash, query: &MoveQuery) {
    spatial_area.remove_object(&query.entity, &query.bounds(query.source));
    spatial_area.insert_object(&query.entity, &query.bounds(query.destination));
}

/// Obtain all nearby players.
pub fn get_nearby(
    world: &World,
    spatial: &SpatialHash,
    player: &Entity,
    range: f64,
) -> Vec<(Entity, Player)> {
    let mut results: Vec<(Entity, Player)> = Vec::new();
    if let Some(pos) = world.get_component::<Position>(player) {
        let range = Bounds::from_vec(pos.loc, pos.size).scaled_center(range);
        let nearby = spatial.query(&range, Some(player));

        for e in nearby.into_iter() {
            if let Some(player) = world.get_component::<Player>(&e) {
                results.push((e, *player));
            }
        }
    }

    results
}
