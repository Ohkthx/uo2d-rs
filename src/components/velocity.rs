use crate::impl_component;

use super::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Velocity(pub Vec2);

impl_component!(Velocity);
