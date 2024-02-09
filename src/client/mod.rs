use std::time::Duration;
use std::{error::Error, thread};

use sdl2::event::Event;
use sdl2::image::{self, InitFlag};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

use crate::packet::{Action, Payload};
use crate::util::exec_rainbow;

mod socket_client;

use self::socket_client::SocketClient;

pub struct Client {
    socket: SocketClient,
}

impl Client {
    /// Creates a new client, holding the socket.
    fn new(socket: SocketClient) -> Client {
        Client { socket }
    }

    /// Starts the client, this begins the remote listerning and graphics.
    pub fn start(address: &str) -> Result<(), Box<dyn Error>> {
        // Create socket and tell the server we are joining.
        let socket = SocketClient::new(address);
        socket.send(Action::ClientJoin, Payload::Empty);

        // Run the SDL2 game loop on the main thread
        let mut client = Client::new(socket);
        client.gameloop()?;

        // Inform server we are quitting.
        client.socket.send(Action::ClientLeave, Payload::Empty);
        std::thread::sleep(Duration::from_millis(250));

        Ok(())
    }

    /// This is responsible for processing the graphics and responses from the remote server.
    fn gameloop(&mut self) -> Result<(), String> {
        let sdl_context = sdl2::init().map_err(|e| e.to_string())?;
        let video_subsystem = sdl_context.video().map_err(|e| e.to_string())?;

        let _image_context = image::init(InitFlag::PNG).map_err(|e| e.to_string())?;

        let window = video_subsystem
            .window("uo2d", 800, 600)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;
        let mut canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        let mut event_pump = sdl_context.event_pump().map_err(|e| e.to_string())?;

        let mut rgb = (0, 0, 0);

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

            // Game rendering and logic here
            rgb = exec_rainbow(rgb, 5);
            canvas.set_draw_color(Color::RGB(rgb.0, rgb.1, rgb.2));

            canvas.clear();
            canvas.present();

            // Process the data from the server if there is any.
            let packets = self.socket.get_packets();
            for packet in packets.into_iter() {
                if let Ok(Some((action, payload))) = self.socket.process_packet(packet) {
                    self.socket.send(action, payload);
                }
            }

            thread::sleep(Duration::from_millis(16));
        }

        Ok(())
    }
}
