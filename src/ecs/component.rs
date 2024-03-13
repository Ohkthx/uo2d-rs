use std::any::{Any, TypeId};

/// Represents a component within an ECS.
pub trait Component: Send + Sync + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

#[macro_export]
macro_rules! impl_component {
    ($($type:ty),*) => {
        $(
            impl $crate::ecs::Component for $type {
                fn as_any(&self) -> &dyn std::any::Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                    self
                }
            }
        )*
    };
}
