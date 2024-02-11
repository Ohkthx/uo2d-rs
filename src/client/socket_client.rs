use std::thread;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
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
                // Connect to the server.
                let stream = TcpStream::connect(addr_clone).await.unwrap();
                let (mut reader, mut writer) = stream.into_split();

                // Handle sending packets to the server.
                let send_task = tokio::spawn(async move {
                    while let Some(packet) = receiver.recv().await {
                        // Convert Packet to bytes and send.
                        if let Err(why) = writer.write_all(&packet.to_bytes()).await {
                            cprintln!("ERROR WRITING {}", why);
                        }
                    }
                });

                // Handle receiving packets from the server.
                let recv_task = tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    loop {
                        let n = reader.read(&mut buf).await.unwrap();
                        if n == 0 {
                            break;
                        }
                        let packet = Packet::from_bytes(&buf[..n]);
                        cache_clone.add(packet);
                    }
                });

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
