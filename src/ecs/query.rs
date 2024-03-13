use std::any::TypeId;

use super::{Component, Entity, World};

/// Tracks the changes for a component.
pub enum ComponentChange<T: Component> {
    /// Component needs to be updated.
    Update(Entity, T),
    /// Component needs to be removed.
    Remove(Entity),
}

impl<T: Component> ComponentChange<T> {
    /// Processes the changes for a component.
    pub fn processor(world: &mut World, changes: Vec<ComponentChange<T>>) {
        for change in changes.into_iter() {
            match change {
                ComponentChange::Remove(entity) => {
                    world.remove_component_by_type_id(entity, &TypeId::of::<T>());
                }
                ComponentChange::Update(entity, vel) => {
                    world.upsert_component(entity, vel);
                }
            }
        }
    }
}
