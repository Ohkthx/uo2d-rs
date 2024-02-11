use std::collections::HashMap;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

use uuid::Uuid;

use crate::{cprintln, util::exec_rainbow};

/// Represents players within the game.
pub struct Player {
    pub uuid: Uuid,
    pub color: (u8, u8, u8),
    pub pos: (i32, i32),
    pub size: u16,
}

impl Player {
    pub fn draw(&self, canvas: &mut WindowCanvas) {
        // Draw the border
        let border_rect = Rect::new(self.pos.0, self.pos.1, self.size as u32, self.size as u32);
        canvas.set_draw_color(Color::RGB(0, 0, 0)); // Black color for the border
        if let Err(why) = canvas.fill_rect(border_rect) {
            cprintln!("Unable to render border for {}: {}", self.uuid, why);
        }

        // Draw the base square on top of the border
        let base_rect = Rect::new(
            self.pos.0 + 2,
            self.pos.1 + 2,
            self.size as u32 - 4,
            self.size as u32 - 4,
        );
        canvas.set_draw_color(Color::RGB(self.color.0, self.color.1, self.color.2));
        if let Err(why) = canvas.fill_rect(base_rect) {
            cprintln!("Unable to render base for {}: {}", self.uuid, why);
        }
    }
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
    pub fn upsert_player(&mut self, uuid: Uuid, position: (i32, i32), size: u16) {
        if let Some(player) = self.players.get_mut(&uuid) {
            player.pos = position;
            player.size = size;
            return;
        }

        self.players.insert(
            uuid,
            Player {
                uuid,
                color: self.next_color,
                pos: position,
                size,
            },
        );
        self.next_color = exec_rainbow(self.next_color, 35);
    }

    /// Removes a player being tracked.
    pub fn remove_player(&mut self, uuid: Uuid) {
        self.players.remove(&uuid);
    }
}
