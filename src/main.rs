mod client;
mod packet;
mod server;
mod util;

use std::{env, error::Error};

use client::Client;
use server::Server;

const ADDRESS: &str = "127.0.0.1:31013";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    // Start either server or client.
    if args.contains(&String::from("--server")) {
        Server::new().start(ADDRESS).await?;
    } else {
        Client::new().start(ADDRESS).await?;
    }

    Ok(())
}
