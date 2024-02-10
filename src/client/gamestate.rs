use std::collections::HashMap;

use uuid::Uuid;

use crate::util::exec_rainbow;

/// Represents players within the game.
pub struct Player {
    pub uuid: Uuid,
    pub color: (u8, u8, u8),
    pub pos: (i32, i32),
}

/// Current tracked state of the game.
pub struct Gamestate {
    pub players: HashMap<Uuid, Player>,
    next_color: (u8, u8, u8),
    pub kill: bool,
}

impl Gamestate {
    /// Initializes the gamestate.
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            next_color: (0, 0, 0),
            kill: false,
        }
    }

    /// Adds a new player to be tracked.
    pub fn add_player(&mut self, uuid: Uuid, position: (i32, i32)) {
        if self.players.contains_key(&uuid) {
            return;
        }

        self.players.insert(
            uuid,
            Player {
                uuid,
                color: self.next_color,
                pos: position,
            },
        );
        self.next_color = exec_rainbow(self.next_color, 35);
    }

    /// Removes a player being tracked.
    pub fn remove_player(&mut self, uuid: Uuid) {
        self.players.remove(&uuid);
    }
}
