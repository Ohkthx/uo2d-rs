use super::{component::Component, entity::Entity};

// A generic storage for components that uses a sparse set for efficient storage and retrieval.
#[derive(Default)]
pub(crate) struct SparseSet {
    // Maps entity ID to index in `components`.
    sparse: Vec<Option<usize>>,
    // Dense storage of components, compact and cache-friendly.
    dense: Vec<Box<dyn Component>>,
    // Mapping from dense index back to entity.
    entities: Vec<Entity>,
}

impl SparseSet {
    pub(crate) fn entities(&self) -> &[Entity] {
        &self.entities
    }

    // Add a component to an entity.
    pub(crate) fn insert(&mut self, entity: Entity, component: Box<dyn Component>) {
        let entity_id = entity.id() as usize;
        // Ensure the sparse vector is large enough to contain the entity ID.
        if entity_id >= self.sparse.len() {
            self.sparse.resize(entity_id + 1, None);
        }
        // Check if the entity already has this component type.
        if let Some(index) = self.sparse[entity_id] {
            // Replace the existing component.
            self.dense[index] = component;
        } else {
            // Add new component.
            self.entities.push(entity);
            self.dense.push(component);
            self.sparse[entity_id] = Some(self.dense.len() - 1);
        }
    }

    // Retrieve a component by entity.
    pub(crate) fn get(&self, entity: &Entity) -> Option<&dyn Component> {
        let entity_id = entity.id() as usize;
        self.sparse
            .get(entity_id)
            .and_then(|&index| index)
            .and_then(|index| {
                self.dense
                    .get(index)
                    .map(|box_dyn_comp| box_dyn_comp.as_ref())
            })
    }

    // Retrieve a mutable reference to a component by entity.
    pub(crate) fn get_mut(&mut self, entity: &Entity) -> Option<&mut dyn Component> {
        let entity_id = entity.id() as usize;
        if let Some(Some(index)) = self.sparse.get(entity_id) {
            // Access the dense array mutably and return a mutable reference to the component.
            self.dense
                .get_mut(*index)
                .map(|box_dyn_comp| box_dyn_comp.as_mut())
        } else {
            None
        }
    }

    // Removes a component associated with an entity.
    pub(crate) fn remove(&mut self, entity: &Entity) {
        let entity_id = entity.id() as usize;
        if entity_id < self.sparse.len() {
            if let Some(dense_index) = self.sparse[entity_id] {
                let last_index = self.dense.len() - 1;
                self.dense.swap(dense_index, last_index);
                self.dense.pop();

                // Remove the entity from the `entities` array in a similar fashion.
                self.entities.swap(dense_index, last_index);
                self.entities.pop();

                // Update the `sparse` array for the entity that was moved.
                if dense_index < self.dense.len() {
                    // Check if there was an element to swap.
                    let swapped_entity_id = self.entities[dense_index].id() as usize;
                    self.sparse[swapped_entity_id] = Some(dense_index);
                }

                // Finally, mark the entity's component as removed in the `sparse` array.
                self.sparse[entity_id] = None;
            }
        }
    }

    // Efficiently iterate over all components.
    #[allow(dead_code)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Box<dyn Component>> {
        self.dense.iter()
    }
}
