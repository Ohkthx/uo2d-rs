use std::error::Error;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::cprintln;
use crate::packet::payloads::PingPayload;
use crate::packet::{Action, Packet, Payload};
use crate::util::get_utc;

/// Negotiates with the server.
pub struct Client {
    uuid: Uuid,
    sender: Option<OwnedWriteHalf>,
}

impl Client {
    /// Create a new instance of the client.
    pub fn new() -> Client {
        Client {
            uuid: Uuid::nil(),
            sender: None,
        }
    }

    /// Starts the client for listening to the server.
    pub async fn start(&mut self, address: &str) -> Result<(), Box<dyn Error>> {
        let shutdown_signals = async {
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to bind SIGINT handler");
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to bind SIGTERM handler");

            tokio::select! {
                _ = sigint.recv() => {
                    cprintln!("SIGINT received.");
                },
                _ = sigterm.recv() => {
                    cprintln!("SIGTERM received.");
                },
            }
        };

        // Initialize connection.
        match TcpStream::connect(address).await {
            Ok(stream) => {
                println!("Successfully connected to server at {}", address);

                tokio::select! {
                    // Incoming data / packets.
                    _ = self.handle(stream) => {},
                    // Shutdown received, notify server.
                    _ = shutdown_signals => {
                        cprintln!("Shutting down.");
                        let packet = Packet::new(Action::ClientLeave, self.uuid, Payload::Empty);
                        self.send(packet).await?;
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            Err(e) => {
                return Err(format!("Failed to connect: {}", e).into());
            }
        }

        Ok(())
    }

    /// Send a packet to ther server.
    pub async fn send(&mut self, packet: Packet) -> Result<(), Box<dyn Error>> {
        if let Some(sender) = self.sender.as_mut() {
            let _ = sender.write_all(&packet.to_bytes()?).await;
        }

        Ok(())
    }

    /// Processes a packet, responding to server if necessary.
    async fn process_packet(&mut self, packet: Packet) -> Result<Option<Packet>, String> {
        let (action, payload) = match packet.action {
            Action::Ping => match packet.payload {
                Payload::Ping(ping) => {
                    cprintln!("PING {} from server", ping.uuid);
                    (Action::Ping, Payload::Ping(PingPayload::new(ping.uuid)))
                }
                _ => return Ok(None),
            },
            Action::Shutdown => match packet.payload {
                Payload::Message(msg) => {
                    cprintln!("{}", msg.message);
                    return Err("Server shutdown.".to_string());
                }
                _ => return Err("Server shutdown.".to_string()),
            },
            Action::Success => {
                self.uuid = packet.uuid;
                return Ok(None);
            }
            Action::Message => match packet.payload {
                Payload::Message(msg) => {
                    cprintln!("{}", msg.message);
                    return Ok(None);
                }
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        Ok(Some(Packet::new(action, packet.uuid, payload)))
    }

    /// Handles all incoming data from the server.
    async fn handle(&mut self, stream: TcpStream) -> Result<(), Box<dyn Error>> {
        let (mut reader, writer) = stream.into_split();
        self.sender = Some(writer);

        // Tell the server we want to join, this gives us our UUID.
        let join = Packet::new(Action::ClientJoin, Uuid::nil(), Payload::Empty);
        self.send(join).await?;

        let mut buffer = Vec::new();
        loop {
            let mut temp_buffer = [0; 512]; // Smaller temporary buffer/
            match reader.read(&mut temp_buffer).await {
                Ok(0) => return Ok(()), // Connection closed.
                Ok(bytes_read) => buffer.extend_from_slice(&temp_buffer[..bytes_read]),
                Err(e) => {
                    println!("Failed to read from stream: {}", e);
                    return Ok(());
                }
            }

            // Convert from bytes to a packet and process it.
            if let Ok(packet) = Packet::from_bytes(&buffer) {
                if let Some(response) = self.process_packet(packet).await? {
                    self.send(response).await?;
                }
            }

            buffer.clear();
        }
    }
}
