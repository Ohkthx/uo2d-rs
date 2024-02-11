mod cache;
mod client;
mod entity;
mod packet;
mod server;
mod spatial_hash;
mod util;

use std::env;
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use client::Client;
use server::Server;

const ADDRESS: &str = "127.0.0.1:31013";

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    // Start either server or client.
    if args.contains(&String::from("--server")) {
        Server::start(ADDRESS)?;
    } else {
        // Start the server instance.
        if args.contains(&String::from("--solo")) {
            let server_address = ADDRESS.to_string();
            std::thread::spawn(move || {
                if let Err(e) = Server::start(&server_address) {
                    eprintln!("Server failed to start: {}", e);
                }
            });

            sleep(Duration::from_secs(1));
        }

        Client::start(ADDRESS)?;
    }

    Ok(())
}
