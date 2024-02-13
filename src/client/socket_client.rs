use std::sync::Arc;
use std::thread;

use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::cache::PacketCacheSync;
use crate::client::packet_processor::processor;
use crate::cprintln;
use crate::packet::{Action, Packet, Payload};

use super::gamestate::Gamestate;

/// Used to communicate to the remove server.
pub struct SocketClient {
    pub uuid: Uuid,
    sender: mpsc::Sender<Packet>,
    packet_cache: PacketCacheSync,
}

impl SocketClient {
    /// Create a new client instance.
    pub fn new(address: &str) -> Self {
        let (sender, mut receiver) = mpsc::channel::<Packet>(32);
        let packet_cache = PacketCacheSync::new(usize::MAX);

        let cache_clone = packet_cache.clone();
        let addr_clone = address.to_string();

        // Launch the asynchronous task.
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let local_addr = "0.0.0.0:0";
                let socket = Arc::new(Mutex::new(UdpSocket::bind(local_addr).await.unwrap()));
                socket.lock().await.connect(addr_clone).await.unwrap();

                // Handle sending packets to the server.
                let send_socket = Arc::clone(&socket);

                let send_task = tokio::spawn(async move {
                    while let Some(packet) = receiver.recv().await {
                        // Convert Packet to bytes and send.
                        let packet_bytes = packet.to_bytes();
                        if let Err(why) = send_socket.lock().await.send(&packet_bytes).await {
                            cprintln!("ERROR SENDING: {}", why);
                        }
                    }
                });

                // Handle receiving packets from the server.
                let recv_socket = Arc::clone(&socket);
                let recv_task = tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    loop {
                        // Temporarily store the result of trying to receive data
                        let recv_result = {
                            let socket = recv_socket.lock().await; // Lock is acquired and immediately dropped after the block
                            socket.try_recv(&mut buf)
                        };

                        if let Ok(n) = recv_result {
                            if n == 0 {
                                break;
                            }

                            cache_clone.add(Packet::from_bytes(&buf[..n]));
                        }
                    }
                });

                // Wait for both tasks to complete
                tokio::try_join!(send_task, recv_task).unwrap();
            });
        });

        Self {
            uuid: Uuid::nil(),
            sender,
            packet_cache,
        }
    }

    /// Send a packet to the server asynchronously.
    pub fn send(&self, action: Action, payload: Payload) {
        let _ = self
            .sender
            .try_send(Packet::new(action, self.uuid, payload));
    }

    /// Retrieve received packets from the cache.
    pub fn get_packets(&self) -> Vec<Packet> {
        self.packet_cache.get_all()
    }

    /// Processes a packet, returns an action and payload if one needs to be sent.
    pub fn process_packet(
        &mut self,
        gamestate: &mut Gamestate,
        packet: Packet,
    ) -> Option<(Action, Payload)> {
        processor(self, gamestate, packet)
    }
}
