use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
};

use super::{component::Component, entity::Entity, sparse_set::SparseSet};

/// Used to construct a new entity with components.
pub struct EntityBuilder<'a> {
    world: &'a mut World,
    entity: Entity,
}

impl<'a> EntityBuilder<'a> {
    // Constructor to create a new EntityBuilder.
    pub fn new(world: &'a mut World, entity: Entity) -> Self {
        Self { world, entity }
    }

    // Add a component to the entity.
    pub fn with<T: Component + 'static>(self, component: T) -> Self {
        self.world.add_component(self.entity, component);
        self
    }

    // Finalize the entity.
    pub fn build(self) -> Entity {
        self.entity
    }
}

/// Represents the world and all components that exist.
pub struct World {
    id: u64,
    components: HashMap<TypeId, Box<dyn Any>>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Creates a new instance of the world.
    pub fn new() -> Self {
        Self {
            id: 0,
            components: HashMap::new(),
        }
    }

    /// Creates a new Id for an entity.
    fn generate_id(&mut self) -> u64 {
        self.id += 1;
        self.id
    }

    /// Registers a new component that can be queried.
    pub fn register_component<T: Component + 'static>(&mut self) {
        self.components
            .insert(TypeId::of::<T>(), Box::<SparseSet>::default());
    }

    /// Spawns a new entity with a optional components.
    pub fn spawn(&mut self) -> EntityBuilder {
        let entity = Entity::new(self.generate_id());
        EntityBuilder::new(self, entity)
    }

    /// Removes and entity and all associated components.
    pub fn despawn(&mut self, entity: &Entity) {
        for component in self.components.values_mut() {
            if let Some(sparse_set) = component.downcast_mut::<SparseSet>() {
                sparse_set.remove(entity);
            }
        }
    }

    /// Obtains all entities.
    pub fn get_entities<T: 'static>(&self) -> HashSet<Entity> {
        let mut entities = HashSet::new();

        if let Some(sparse_set_any) = self.components.get(&TypeId::of::<T>()) {
            if let Some(sparse_set) = sparse_set_any.downcast_ref::<SparseSet>() {
                for entity in sparse_set.entities() {
                    entities.insert(*entity);
                }
            }
        }

        entities
    }

    /// Adds a new component to the entity.
    pub fn add_component<T: Component + 'static>(&mut self, entity: Entity, component: T) {
        if let Some(any_set) = self.components.get_mut(&TypeId::of::<T>()) {
            if let Some(set) = any_set.downcast_mut::<SparseSet>() {
                set.insert(entity, Box::new(component));
            }
        }
    }

    /// Obtains a specific component for an entity.
    pub fn get_component<T: Component + 'static>(&self, entity: &Entity) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|any_sparse_set| {
                if let Some(sparse_set) = any_sparse_set.downcast_ref::<SparseSet>() {
                    sparse_set
                        .get(entity)
                        .and_then(|comp| comp.as_any().downcast_ref::<T>())
                } else {
                    None
                }
            })
    }

    /// Obtains a mutable component for an entity.
    pub fn get_component_mut<T: Component + 'static>(&mut self, entity: &Entity) -> Option<&mut T> {
        // Attempt to get a mutable reference to the SparseSet from the components HashMap.
        self.components
            .get_mut(&TypeId::of::<T>())
            .and_then(|any_sparse_set| {
                // Downcast to SparseSet mutably.
                if let Some(sparse_set) = any_sparse_set.downcast_mut::<SparseSet>() {
                    // Attempt to get a mutable reference to the component.
                    sparse_set
                        .get_mut(entity)
                        .and_then(|comp| comp.as_any_mut().downcast_mut::<T>())
                } else {
                    None
                }
            })
    }

    /// Removes a component attached to an entity.
    pub fn remove_component<T: Component + 'static>(&mut self, entity: Entity) {
        self.remove_component_by_type_id(entity, &TypeId::of::<T>());
    }

    /// Removes a component based on its type Id.
    pub fn remove_component_by_type_id(&mut self, entity: Entity, type_id: &TypeId) {
        if let Some(any_sparse_set) = self.components.get_mut(type_id) {
            if let Some(sparse_set) = any_sparse_set.downcast_mut::<SparseSet>() {
                sparse_set.remove(&entity);
            }
        }
    }

    // Generic method to replace or insert a component for a given entity.
    pub fn upsert_component<T: Component + 'static>(&mut self, entity: Entity, new_component: T) {
        // Get the TypeId for the component.
        let type_id = TypeId::of::<T>();

        // Check if the SparseSet for this component type exists; if not, create it.
        let component_set = self
            .components
            .entry(type_id)
            .or_insert_with(|| Box::<SparseSet>::default());

        // Downcast the Box<dyn Any> to a mutable reference to SparseSet.
        if let Some(sparse_set) = component_set.downcast_mut::<SparseSet>() {
            sparse_set.remove(&entity);
            sparse_set.insert(entity, Box::new(new_component) as Box<dyn Component>);
        } else {
            // Handle the error case where downcast_mut fails (this should not happen in practice if your setup is correct)
            eprintln!("Failed to downcast to SparseSet for component replacement.");
        }
    }

    #[allow(dead_code)]
    /// Updates several components.
    pub fn update_components<T: Component + 'static>(&mut self, updates: Vec<(Entity, T)>) {
        for (entity, new_comp) in updates {
            // Update component.
            if let Some(old_comp) = self.get_component_mut::<T>(&entity) {
                *old_comp = new_comp;
            } else {
                // Create if component is missing.
                self.upsert_component::<T>(entity, new_comp);
            }
        }
    }

    /// Queries all entities and components of a specified type.
    pub fn query1<T: Component + 'static>(&self) -> Vec<(Entity, &T)> {
        let mut results = Vec::new();

        if let Some(sparse_set_any) = self.components.get(&TypeId::of::<T>()) {
            if let Some(sparse_set) = sparse_set_any.downcast_ref::<SparseSet>() {
                // Iterate through entities in the sparse set.
                for entity in sparse_set.entities() {
                    if let Some(component) = self.get_component::<T>(entity) {
                        results.push((*entity, component));
                    }
                }
            }
        }

        results
    }

    // Method to query entities with multiple component types
    pub fn query2<T: Component + 'static, U: Component + 'static>(&self) -> Vec<(Entity, &T, &U)> {
        let mut results = Vec::new();

        if let (Some(t_sparse_set_any), Some(u_sparse_set_any)) = (
            self.components.get(&TypeId::of::<T>()),
            self.components.get(&TypeId::of::<U>()),
        ) {
            if let (Some(t_sparse_set), Some(u_sparse_set)) = (
                t_sparse_set_any.downcast_ref::<SparseSet>(),
                u_sparse_set_any.downcast_ref::<SparseSet>(),
            ) {
                // Intersection of entities that have both components.
                let t_entities = &t_sparse_set.entities();
                let u_entities = &u_sparse_set.entities();

                let intersection: Vec<Entity> = t_entities
                    .iter()
                    .filter(|&entity| u_entities.contains(entity))
                    .cloned()
                    .collect();

                for entity in intersection {
                    if let (Some(t_component), Some(u_component)) = (
                        self.get_component::<T>(&entity),
                        self.get_component::<U>(&entity),
                    ) {
                        results.push((entity, t_component, u_component));
                    }
                }
            }
        }

        results
    }

    /// Queries based on three components obtaining all matching components and entities.
    pub fn query3<T: Component + 'static, U: Component + 'static, V: Component + 'static>(
        &self,
    ) -> Vec<(Entity, &T, &U, &V)> {
        let mut results = Vec::new();

        // Attempt to retrieve the SparseSets for each component type.
        if let (Some(t_sparse_set_any), Some(u_sparse_set_any), Some(v_sparse_set_any)) = (
            self.components.get(&TypeId::of::<T>()),
            self.components.get(&TypeId::of::<U>()),
            self.components.get(&TypeId::of::<V>()),
        ) {
            if let (Some(t_sparse_set), Some(u_sparse_set), Some(v_sparse_set)) = (
                t_sparse_set_any.downcast_ref::<SparseSet>(),
                u_sparse_set_any.downcast_ref::<SparseSet>(),
                v_sparse_set_any.downcast_ref::<SparseSet>(),
            ) {
                // Find the intersection of entities that have all three components.
                let t_entities = t_sparse_set.entities();
                let u_entities = u_sparse_set.entities();
                let v_entities = v_sparse_set.entities();

                let intersection: Vec<Entity> = t_entities
                    .iter()
                    .filter(|&entity| u_entities.contains(entity) && v_entities.contains(entity))
                    .cloned()
                    .collect();

                for entity in intersection {
                    if let (Some(t_component), Some(u_component), Some(v_component)) = (
                        self.get_component::<T>(&entity),
                        self.get_component::<U>(&entity),
                        self.get_component::<V>(&entity),
                    ) {
                        results.push((entity, t_component, u_component, v_component));
                    }
                }
            }
        }

        results
    }
}
