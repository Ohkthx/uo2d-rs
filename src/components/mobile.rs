use uuid::Uuid;

use crate::impl_component;

#[derive(Debug, Clone, Copy)]
pub struct Player(Uuid);

impl Player {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn uuid(&self) -> &Uuid {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Projectile;

impl_component!(Player, Projectile);
