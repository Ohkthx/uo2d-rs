use std::error::Error;
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;

pub use socket_server::SocketServer;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::cache::PacketCacheAsync;
use crate::packet::PacketConfiguration;
use crate::{sprintln, util::get_now};

use self::gamestate::Gamestate;

mod gamestate;
mod packet_processor;
pub mod socket_server;

/// Holds all of the relevant client information for send/recving packets.
#[derive(Clone)]
pub(crate) struct Client {
    pub(crate) uuid: Uuid,
    pub addr: SocketAddr,
    ping_id: Uuid,
    last_ping: u64,
}

impl Client {
    /// Create a new instance of the client to be tracked.
    pub fn new(uuid: Uuid, addr: SocketAddr) -> Client {
        Client {
            uuid,
            addr,
            ping_id: Uuid::nil(),
            last_ping: get_now(),
        }
    }
}

pub struct Server {}

impl Server {
    /// Starts the client, this begins the remote listerning and graphics.
    pub fn start(address: &str) -> Result<(), Box<dyn Error>> {
        let (tx, rx) = mpsc::channel::<PacketConfiguration>(32);

        // Create socket and listen for connections.
        let packet_cache = PacketCacheAsync::new(1);
        let addr_clone = address.to_string();

        let cache = packet_cache.clone();
        std::thread::spawn(move || {
            if let Err(why) = SocketServer::start(&addr_clone, rx, cache) {
                sprintln!("ERROR stopping socket server {}", why);
            }
        });

        let handle = std::thread::spawn(move || {
            // Create a new Tokio runtime
            let rt = Runtime::new().expect("Failed to create a runtime");

            // Block on the async `start` function using the runtime
            rt.block_on(async {
                let mut gamestate = Gamestate::new(tx, packet_cache, (800, 800));
                gamestate.start().await;
            });
        });

        if handle.join().is_err() {
            sprintln!("ERROR while joining the thread.");
        }

        sleep(Duration::from_secs(1));
        Ok(())
    }
}
