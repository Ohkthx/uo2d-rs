use std::hash::Hash;

use serde::{Deserialize, Serialize};

/// Represents and Entity within an ECS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Entity(u64);

impl Entity {
    pub const INVALID: Self = Entity(u64::MAX);

    pub fn new(id: u64) -> Self {
        Entity(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

impl Hash for Entity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl std::fmt::Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
