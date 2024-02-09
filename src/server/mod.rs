use std::net::SocketAddr;

pub use socket_server::SocketServer;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::util::get_now;

mod packet_processor;
pub mod socket_server;

/// Holds all of the relevant client information for send/recving packets.
#[derive(Clone)]
pub(crate) struct Client {
    uuid: Uuid,
    _addr: SocketAddr,
    tx: mpsc::Sender<Vec<u8>>,
    ping_id: Uuid,
    last_ping: u64,
}

impl Client {
    /// Create a new instance of the client to be tracked.
    pub fn new(uuid: Uuid, _addr: SocketAddr, tx: mpsc::Sender<Vec<u8>>) -> Client {
        Client {
            uuid,
            _addr,
            tx,
            ping_id: Uuid::nil(),
            last_ping: get_now(),
        }
    }
}
