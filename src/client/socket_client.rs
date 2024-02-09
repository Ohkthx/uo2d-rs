use std::sync::{Arc, Mutex};
use std::thread;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::cprintln;
use crate::packet::payloads::PingPayload;
use crate::packet::{Action, Packet, Payload};

/// Used to communicate to the remove server.
pub struct SocketClient {
    pub uuid: Uuid,
    sender: mpsc::Sender<Packet>,
    received_packets: Arc<Mutex<Vec<Packet>>>,
}

impl SocketClient {
    /// Create a new client instance.
    pub fn new(address: &str) -> Self {
        let (sender, mut receiver) = mpsc::channel::<Packet>(32);
        let received_packets = Arc::new(Mutex::new(Vec::new()));

        let received_packets_clone = Arc::clone(&received_packets);
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
                        received_packets_clone.lock().unwrap().push(packet);
                    }
                });

                tokio::try_join!(send_task, recv_task).unwrap();
            });
        });

        Self {
            uuid: Uuid::nil(),
            sender,
            received_packets,
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
        let mut received = self.received_packets.lock().unwrap();
        std::mem::take(&mut *received)
    }

    /// Processes a packet, returns an action and payload if one needs to be sent.
    pub fn process_packet(&mut self, packet: Packet) -> Result<Option<(Action, Payload)>, String> {
        let (action, payload) = match packet.action() {
            Action::Ping => match packet.payload() {
                Payload::Ping(ping) => {
                    cprintln!("PING {} from server", ping.uuid);
                    (Action::Ping, Payload::Ping(PingPayload::new(ping.uuid)))
                }
                _ => return Ok(None),
            },
            Action::Shutdown => match packet.payload() {
                Payload::Message(msg) => {
                    cprintln!("{}", msg.message);
                    return Err(String::from("Server shutdown."));
                }
                _ => {
                    return Err(String::from("Server shutdown."));
                }
            },
            Action::Success => {
                self.uuid = packet.uuid();
                return Ok(None);
            }
            Action::Message => match packet.payload() {
                Payload::Message(msg) => {
                    cprintln!("{}", msg.message);
                    return Ok(None);
                }
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        Ok(Some((action, payload)))
    }
}
