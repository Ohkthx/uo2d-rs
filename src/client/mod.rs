use std::path::Path;
use std::time::Duration;
use std::{error::Error, thread};

use sdl2::event::Event;
use sdl2::image::{self, InitFlag, LoadTexture};
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use uuid::Uuid;

use crate::cprintln;
use crate::object::Position;
use crate::packet::payloads::MovementPayload;
use crate::packet::{Action, Payload};

mod gamestate;
mod packet_processor;
mod socket_client;

use self::gamestate::{Entity, Gamestate};
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

    fn player(&self) -> &Entity {
        self.gamestate.get_entity(&self.uuid()).unwrap()
    }

    /// Starts the client, this begins the remote listerning and graphics.
    pub fn start(address: &str) -> Result<(), Box<dyn Error>> {
        // Create socket and tell the server we are joining.
        let socket = SocketClient::new(address);
        let start: Position = (
            WINDOW_DIMENSIONS.0 as i32 / 2,
            WINDOW_DIMENSIONS.1 as i32 / 2,
            1,
        );

        let mut client = Self::new(socket);
        client.send(
            Action::ClientJoin,
            Payload::Movement(MovementPayload::new((32, 32), start, (0.0, 0.0), 0.0)),
        );

        // Wait until we have authenticated.
        while client.uuid() == Uuid::nil() {
            let packets = client.socket.get_packets();
            for packet in packets.into_iter() {
                client.socket.process_packet(&mut client.gamestate, packet);
            }
            std::thread::sleep(client.gamestate.timers.client_tick_time());
        }

        // Add the client as a player.
        cprintln!("Player UUID: {}", client.uuid());
        client
            .gamestate
            .upsert_entity(client.uuid(), start, (32, 32));

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
            .window(
                &format!("uo2d - {}", self.uuid()),
                WINDOW_DIMENSIONS.0,
                WINDOW_DIMENSIONS.1,
            )
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;
        let mut canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        let texture_creator = canvas.texture_creator();
        let background_texture =
            texture_creator.load_texture(Path::new("assets/background.png"))?;

        // Get image (texture) dimensions
        let TextureQuery {
            width: img_width,
            height: img_height,
            ..
        } = background_texture.query();

        // Get window size
        let (win_width, win_height) = canvas.window().size();

        // Calculate position to center the image
        let center_x = (win_width as i32 - img_width as i32) / 2;
        let center_y = (win_height as i32 - img_height as i32) / 2;
        let mut bg = Rect::new(center_x, center_y, img_width, img_height);

        // Color management.
        // let mut background = (0, 0, 0);

        let mut event_pump = sdl_context.event_pump().map_err(|e| e.to_string())?;
        let mut target_pos: Option<(i32, i32)> = None;
        let move_speed = 5.0;

        'running: loop {
            for timer in self.gamestate.timers.update() {
                cprintln!("Expired: {:?}", timer);
            }

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    Event::MouseButtonDown {
                        x, y, mouse_btn, ..
                    } => {
                        if mouse_btn == MouseButton::Left {
                            target_pos = Some((x, y)); // Update target position on left mouse click
                        }
                    }
                    _ => {}
                }
            }

            // Check the current state of the keyboard
            let mut trajectory: (f32, f32) = (0.0, 0.0);

            let keyboard_state = event_pump.keyboard_state();
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::W) {
                trajectory.1 = -1.0; // Move up
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::A) {
                trajectory.0 = -1.0; // Move left
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::S) {
                trajectory.1 = 1.0; // Move down
            }
            if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::D) {
                trajectory.0 = 1.0; // Move right
            }

            if trajectory != (0.0, 0.0) {
                target_pos = None;
            } else if let Some(target) = target_pos {
                let player = self.player(); // Assuming this retrieves a mutable reference to the player
                let (px, py) = (player.position.0 as f32, player.position.1 as f32);
                let (tx, ty) = (target.0 as f32, target.1 as f32);

                if (px - tx).abs() > move_speed || (py - ty).abs() > move_speed {
                    // Calculate direction vector
                    let dx = tx - px;
                    let dy = ty - py;
                    let mag = (dx.powi(2) + dy.powi(2)).sqrt();

                    // Calculate and store trajectory vector as a unit vector
                    trajectory = (dx / mag, dy / mag);
                } else {
                    target_pos = None; // Clear target position
                    trajectory = (0.0, 0.0); // Clear trajectory
                }
            }

            // Produces a packet that we have moved to send to server.
            if trajectory != (0.0, 0.0) {
                let p = self.player();
                self.send(
                    Action::Movement,
                    Payload::Movement(MovementPayload::new(
                        p.size, p.position, trajectory, move_speed,
                    )),
                );
            }

            canvas.clear();
            canvas.set_draw_color(Color::BLACK);

            // Renders the background and gamestate entities.
            if let Some(player) = self.gamestate.get_entity(&self.uuid()) {
                let offset = player.center_offset(WINDOW_DIMENSIONS);

                // Move the background / map.
                bg.set_x(center_x - offset.0);
                bg.set_y(center_y - offset.1);
                canvas.copy(&background_texture, None, Some(bg))?;

                self.gamestate.draw(&mut canvas, offset);
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

            thread::sleep(self.gamestate.timers.client_tick_time());
        }

        Ok(())
    }
}
