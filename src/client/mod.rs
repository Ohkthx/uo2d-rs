use std::time::Duration;
use std::{error::Error, thread};

use sdl2::event::Event;
use sdl2::image::{self, InitFlag};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use uuid::Uuid;

use crate::cprintln;
use crate::packet::payloads::MovementPayload;
use crate::packet::{Action, Payload};
use crate::util::exec_rainbow;

mod gamestate;
mod packet_processor;
mod socket_client;

use self::gamestate::Gamestate;
use self::socket_client::SocketClient;

const WINDOW_DIMENSIONS: (u32, u32) = (800, 800);

pub struct Client {
    socket: SocketClient,
    gamestate: Gamestate,
}

impl Client {
    /// Creates a new client, holding the socket.
    fn new(socket: SocketClient) -> Self {
        Self {
            socket,
            gamestate: Gamestate::new(),
        }
    }

    /// Wraps sending packets.
    fn send(&self, action: Action, payload: Payload) {
        self.socket.send(action, payload)
    }

    fn uuid(&self) -> Uuid {
        self.socket.uuid
    }

    /// Starts the client, this begins the remote listerning and graphics.
    pub fn start(address: &str) -> Result<(), Box<dyn Error>> {
        // Create socket and tell the server we are joining.
        let socket = SocketClient::new(address);
        let mut client = Self::new(socket);
        client.send(
            Action::ClientJoin,
            Payload::Movement(MovementPayload::new((
                WINDOW_DIMENSIONS.0 as i32 / 2,
                WINDOW_DIMENSIONS.1 as i32 / 2,
            ))),
        );

        // Wait until we have authenticated.
        while client.uuid() == Uuid::nil() {
            let packets = client.socket.get_packets();
            for packet in packets.into_iter() {
                client.socket.process_packet(&mut client.gamestate, packet);
            }
            std::thread::sleep(Duration::from_millis(16));
        }

        // Add the client as a player.
        cprintln!("Player UUID: {}", client.uuid());
        client.gamestate.add_player(client.uuid(), (400, 400));

        // Run the SDL2 game loop on the main thread.
        client.gameloop()?;

        // Inform server we are quitting.
        client.send(Action::ClientLeave, Payload::Empty);
        std::thread::sleep(Duration::from_millis(250));

        Ok(())
    }

    /// This is responsible for processing the graphics and responses from the remote server.
    fn gameloop(&mut self) -> Result<(), String> {
        let sdl_context = sdl2::init().map_err(|e| e.to_string())?;
        let video_subsystem = sdl_context.video().map_err(|e| e.to_string())?;

        let _image_context = image::init(InitFlag::PNG).map_err(|e| e.to_string())?;

        let window = video_subsystem
            .window("uo2d", WINDOW_DIMENSIONS.0, WINDOW_DIMENSIONS.1)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;
        let mut canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        // Define the initial position and size of the square
        let square_size = 50; // Size of the square
        let mut square_pos = (
            WINDOW_DIMENSIONS.0 as i32 / 2,
            WINDOW_DIMENSIONS.1 as i32 / 2,
        ); // Center of the window

        // Color management.
        let mut background = (0, 0, 0);

        let mut event_pump = sdl_context.event_pump().map_err(|e| e.to_string())?;
        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    _ => {}
                }
            }

            let old_pos = square_pos;
            // Check the current state of the keyboard
            let keyboard_state = event_pump.keyboard_state();
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::W) {
                square_pos.1 -= 10; // Move up
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::A) {
                square_pos.0 -= 10; // Move left
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::S) {
                square_pos.1 += 10; // Move down
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::D) {
                square_pos.0 += 10; // Move right
            }

            if old_pos != square_pos {
                self.send(
                    Action::Movement,
                    Payload::Movement(MovementPayload::new(old_pos)),
                );
            }

            // Update the position.
            if let Some(player) = self.gamestate.players.get_mut(&self.uuid()) {
                player.pos = square_pos;
            }

            // Game rendering and logic here
            background = exec_rainbow(background, 5);
            canvas.set_draw_color(Color::RGB(background.0, background.1, background.2));
            canvas.clear();

            // Draw all players
            for p in self.gamestate.players.values() {
                let square = Rect::new(p.pos.0, p.pos.1, square_size, square_size);
                canvas.set_draw_color(Color::RGB(p.color.0, p.color.1, p.color.2));
                canvas.fill_rect(square).map_err(|e| e.to_string())?;
            }

            canvas.present();

            // Process the data from the server if there is any.
            let packets = self.socket.get_packets();
            for packet in packets.into_iter() {
                if let Some((action, payload)) =
                    self.socket.process_packet(&mut self.gamestate, packet)
                {
                    self.send(action, payload);
                }
            }

            if self.gamestate.kill {
                break 'running;
            }

            thread::sleep(Duration::from_millis(16));
        }

        Ok(())
    }
}
