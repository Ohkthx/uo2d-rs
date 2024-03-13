use std::path::Path;
use std::time::Duration;
use std::{error::Error, thread};

use sdl2::image::{self, InitFlag, LoadTexture};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use uuid::Uuid;

use crate::components::{Bounds, Vec2, Vec3};
use crate::cprintln;
use crate::entities::{Camera, Mobile};
use crate::packet::payloads::MovementPayload;
use crate::packet::{Action, Payload};

mod gamestate;
mod input;
mod packet_processor;
mod socket_client;

use self::gamestate::Gamestate;
use self::input::Input;
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

    fn player(&self) -> &Mobile {
        self.gamestate
            .get_mobile(&self.gamestate.get_player())
            .unwrap()
    }

    /// Starts the client, this begins the remote listerning and graphics.
    pub fn start(address: &str) -> Result<(), Box<dyn Error>> {
        // Create socket and tell the server we are joining.
        let socket = SocketClient::new(address);

        let mut client = Self::new(socket);
        client.send(Action::ClientJoin, Payload::Empty);

        // Wait until we have authenticated.
        while client.uuid() == Uuid::nil() {
            let packets = client.socket.get_packets();
            for packet in packets.into_iter() {
                client.socket.process_packet(&mut client.gamestate, packet);
            }
            std::thread::sleep(client.gamestate.timers.client_tick_time());
        }

        // Add the client as a player.
        cprintln!(
            "Player [{}] UUID: {}",
            client.gamestate.get_player(),
            client.uuid()
        );

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

        // Create the camera.
        let mut camera = Camera::new(
            Vec3::ORIGIN,
            Vec2::new(
                canvas.window().size().0 as f64,
                canvas.window().size().1 as f64,
            ),
        );

        // Position the camera where the player is centered.
        camera.center_on(self.player().position());

        // Calculate position to center the image
        let center_x = (camera.bounding_box().width() as i32 - img_width as i32) / 2;
        let center_y = (camera.bounding_box().height() as i32 - img_height as i32) / 2;
        let mut bg = Rect::new(center_x, center_y, img_width, img_height);

        let mut event_pump = sdl_context.event_pump().map_err(|e| e.to_string())?;
        let mut input = Input::default();
        input.mouse.set_delay(10);
        let mut held_move: bool = false;

        let move_speed = 32.0;

        'running: loop {
            for timer in self.gamestate.timers.update() {
                cprintln!("Expired: {:?}", timer);
            }

            // Process the data from the server if there is any.
            let packets = self.socket.get_packets();
            for packet in packets.into_iter() {
                if let Some((action, payload)) =
                    self.socket.process_packet(&mut self.gamestate, packet)
                {
                    self.send(action, payload);
                }
            }

            // Most recent version of player, update camera.
            let player = self.player();
            camera.center_on(player.position());

            canvas.clear();
            canvas.set_draw_color(Color::BLACK);

            // Renders the background and gamestate entities.
            // Move the background / map.
            let offset = camera.center_offset(&player.position());
            bg.set_x(offset.x().round() as i32);
            bg.set_y(offset.y().round() as i32);
            canvas.copy(&background_texture, None, Some(bg))?;

            self.gamestate.draw(&mut canvas, &camera);
            canvas.present();

            // Update the input tracker.
            let mut velocity: Vec2 = Vec2::ORIGIN;
            input.update(&mut event_pump);
            if self.gamestate.kill || input.keyboard.esc_pressed {
                break 'running;
            } else if input.mouse.left_held() {
                held_move = true;
            }

            // Update the movement towards the mouse pointer.
            let mut move_to: Option<Vec2> = None;
            let mut stopped: bool = false;
            if input.mouse.left_clicked() || input.mouse.left_held() {
                if let Some(target) = input.mouse.last_target {
                    let (x, y) = target.as_tuple();
                    let (dx, dy) = (x - camera.true_center().x(), y - camera.true_center().y());
                    move_to = Some(Vec2::new(
                        player.position().x() + dx,
                        player.position().y() + dy,
                    ));
                }
            } else if !input.mouse.left_held() && held_move {
                // Let go and stop movement.
                held_move = false;
                move_to = None;
                stopped = true; // Used to send no velocity to server.
            }

            // Update the projectile towards the mouse pointer.
            let mut projectile: Vec2 = Vec2::ORIGIN;
            if input.mouse.right_clicked() || input.mouse.right_held() {
                if let Some(target) = input.mouse.last_target {
                    let (x, y) = target.as_tuple();
                    let bb = player.bounding_box();
                    let (dx, dy) = (
                        x - camera.true_center().x() - bb.width() / 2.,
                        y - camera.true_center().y() - bb.height() / 2.,
                    );
                    let mut focus = Some(Vec2::new(
                        player.position().x() + dx,
                        player.position().y() + dy,
                    ));

                    projectile = get_velocity(player.position(), &mut focus);
                }
            }

            // Calculate movement based on keyboard actions.
            if input.keyboard.movement_pressed() {
                if input.keyboard.w_pressed {
                    velocity.set_y(-move_speed); // Move up
                }
                if input.keyboard.a_pressed {
                    velocity.set_x(-move_speed); // Move left
                }
                if input.keyboard.s_pressed {
                    velocity.set_y(move_speed); // Move down
                }
                if input.keyboard.d_pressed {
                    velocity.set_x(move_speed); // Move right
                }

                move_to = None; // Override the mouse clicking.
            } else if move_to.is_some() {
                velocity = get_velocity(player.position(), &mut move_to);
            }

            // Produces a packet that we have moved to send to server or that we wish to stop movement.
            if velocity != Vec2::ORIGIN && (move_to.is_some() || input.keyboard.movement_pressed())
                || stopped
            {
                self.send(
                    Action::Movement,
                    Payload::Movement(MovementPayload::new(
                        player.entity,
                        player.size(),
                        player.position(),
                        velocity,
                    )),
                );
            }

            if projectile != Vec2::ORIGIN {
                let area = Bounds::from_vec(player.position(), player.size());
                let size = Vec2::new(16., 16.);
                let loc = place_outside(&area, projectile, size);

                self.send(
                    Action::Projectile,
                    Payload::Movement(MovementPayload::new(player.entity, size, loc, projectile)),
                );
            }

            thread::sleep(
                self.gamestate
                    .timers
                    .client_tick_time()
                    .saturating_sub(self.gamestate.timers.tick_time()),
            );
        }

        Ok(())
    }
}

/// Obtains the velocity required to move between start and target.
fn get_velocity(start: Vec3, target: &mut Option<Vec2>) -> Vec2 {
    if let Some(tar) = target {
        let (px, py) = (start.x(), start.y());
        let (tx, ty) = tar.as_tuple();

        // Velocity required.
        let vel = Vec2::new(tx - px, ty - py);
        if vel.length() < 1.0 {
            // Set velocity to 0 if we are already close by.
            *target = None;
            Vec2::new(0., 0.)
        } else {
            vel
        }
    } else {
        // Set velocity to 0 if there is not target.
        Vec2::new(0., 0.)
    }
}

/// Gets the nearest coordinates that an object of `size` can exist in relation to the current object at the specified velocity.
pub fn place_outside(mobile: &Bounds, velocity: Vec2, size: Vec2) -> Vec3 {
    let center: Vec2 = mobile.center_2d(); // Center of hitbox coordinate.
    let min_dist: f64 = center.distance(&mobile.top_left_2d()); // Center to top corner (furthest)
    let (dx, dy) = size.apply_scalar(0.5).as_tuple();

    // Get normalize the velocity.
    let mut direction = velocity.normalize();

    // Calculate the additional distance needed to place the object outside, considering its size.
    let extra_dist = (size.x().max(size.y()) / 2.0) + min_dist;
    direction = direction.scaled(extra_dist);

    // Calculate the new position in the direction of the velocity.
    let new_pos = Vec2::new(
        center.x() + direction.x() - dx,
        center.y() + direction.y() - dy,
    );
    Vec3::new(new_pos.x(), new_pos.y(), 1.)
}
