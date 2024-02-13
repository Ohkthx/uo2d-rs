use std::collections::HashSet;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::runtime;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::{interval, sleep};
use uuid::Uuid;

use crate::cache::{ClientCache, PacketCacheAsync};
use crate::packet::payloads::{MessagePayload, UuidPayload};
use crate::packet::{Action, BroadcastScope, Packet, PacketConfiguration, Payload};
use crate::server::packet_processor::process_packet;
use crate::server::Client;
use crate::sprintln;
use crate::util::get_now;

const HEARTBEAT_INTERVAL: u64 = 5;
const MAX_HEARTBEAT_INTERVAL: u64 = HEARTBEAT_INTERVAL * 3;

/// Server instance responsible for managing clients and send/recving updates.
pub struct SocketServer {
    /// The socket to send and receive from.
    socket: UdpSocket,
    /// Current active clients.
    client_cache: ClientCache,
    /// Cached packets for the gamestate.
    packet_cache: PacketCacheAsync,
}

impl SocketServer {
    fn new(socket: UdpSocket, packet_cache: PacketCacheAsync) -> Self {
        Self {
            socket,
            client_cache: ClientCache::new(),
            packet_cache,
        }
    }

    /// Starts the server for listening for incoming connections.
    pub fn start(
        address: &str,
        receiver: Receiver<PacketConfiguration>,
        cache: PacketCacheAsync,
    ) -> Result<(), Box<dyn Error>> {
        let rt = runtime::Runtime::new()?;
        // Use `block_on` to block the current thread until the future completes.
        rt.block_on(async move {
            let socket = UdpSocket::bind(address)
                .await
                .expect("Failed to bind to address");
            sprintln!("Listening on {}", address);

            let server = Self::new(socket, cache);
            if let Err(why) = server.async_main(receiver).await {
                eprintln!("ERROR: {}", why);
            };
        });

        Ok(())
    }

    async fn async_main(
        &self,
        mut gamestate_rx: Receiver<PacketConfiguration>,
    ) -> Result<(), Box<dyn Error>> {
        // Channels for send/recving meessages from packet processor.
        let (mut handler_tx, mut handler_rx) = mpsc::channel::<Vec<u8>>(100);

        let mut buf = vec![0; 1024];
        let mut ping_interval = interval(Duration::from_secs(HEARTBEAT_INTERVAL));

        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to bind SIGINT handler");
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to bind SIGTERM handler");

        'listener: loop {
            tokio::select! {
                // Obtains data from the socket.
                result = self.socket.recv_from(&mut buf) => self.client_receiver(&mut buf, result, &mut handler_tx).await,
                // Sends the heartbeat to all clients.
                _ = ping_interval.tick() => self.send_heartbeat().await,
                // Packet from the gamestate that gets forwarded to clients.
                packet = gamestate_rx.recv() => self.gamestate_receiver(packet).await,
                // Message from the packet processor, updates user last ping status.
                packet = handler_rx.recv() => self.packet_processor_receiver(packet).await,
                // Shutdown signal received.
                _ = sigint.recv() => break 'listener,
                _ = sigterm.recv() => break 'listener,
            }
        }

        self.shutdown().await;
        Ok(())
    }

    /// Handles packet data coming from remote clients.
    async fn client_receiver(
        &self,
        buf: &mut [u8],
        result: Result<(usize, SocketAddr), std::io::Error>,
        handler_tx: &mut Sender<Vec<u8>>,
    ) {
        if let Ok((size, addr)) = result {
            // Process the incoming packet from the client.
            let packet = Packet::from_bytes(&buf[..size]);
            let uuid = if let Some(uuid) = self.client_cache.get_uuid(&addr).await {
                uuid
            } else {
                // Register a new client.
                let uuid = Uuid::new_v4();
                self.client_cache.add(Client::new(uuid, addr)).await;
                uuid
            };

            // Process and respond to the packet.
            let packet_config = process_packet(&self.packet_cache, handler_tx, uuid, packet).await;
            self.send_configuration(packet_config).await
        }
    }

    /// Handles packets coming from the local gamestate.
    async fn gamestate_receiver(&self, packet: Option<PacketConfiguration>) {
        if let Some(packet_config) = packet {
            self.send_configuration(packet_config).await
        }
    }

    /// Handles packets coming from the packet processor.
    async fn packet_processor_receiver(&self, handler_message: Option<Vec<u8>>) {
        if let Some(message) = handler_message {
            let packet: Packet = Packet::from_bytes(&message);
            if let Payload::Uuid(ping) = packet.payload() {
                if let Some(client) = self.client_cache.lock().await.get_mut(&packet.uuid()) {
                    if client.ping_id == ping.uuid {
                        client.last_ping = get_now();
                    }
                }
            }
        }
    }

    /// Broadcasts the server shutting down to all clients.
    async fn shutdown(&self) {
        sprintln!("Shutting down.");
        let packet = Packet::new(
            Action::Shutdown,
            Uuid::nil(),
            Payload::Message(MessagePayload::new("Server is shutting down.")),
        );

        self.packet_cache.add(packet.clone()).await;
        if let Err(why) = self.broadcast(packet, None).await {
            sprintln!("ERROR while shutting down: {}", why);
        }
        sleep(Duration::from_secs(1)).await;
    }

    /// Sends a packet for the clients to respond to, ensures they are still alive.
    async fn send_heartbeat(&self) {
        // UUID they must respond with.
        let ping_id = Uuid::new_v4();

        let ping_packet = Packet::new(
            Action::Ping,
            Uuid::new_v4(),
            Payload::Uuid(UuidPayload::new(ping_id)),
        );

        // Update and clean the clients.
        {
            let mut expired: HashSet<Uuid> = HashSet::new();
            let clients = self.client_cache.clone();
            let now = get_now();

            for (_, client) in clients.lock().await.iter_mut() {
                if now - client.last_ping > MAX_HEARTBEAT_INTERVAL {
                    expired.insert(client.uuid);
                } else {
                    client.ping_id = ping_id;
                }
            }

            // Remove the expired clients.
            for uuid in expired {
                sprintln!("EXPIRED SESSION: {}", uuid);
                clients.remove(&uuid).await;

                let packet = Packet::new(Action::ClientLeave, uuid, Payload::Empty);
                self.packet_cache.add(packet.clone()).await;
                if let Err(why) = self.broadcast(packet, None).await {
                    sprintln!("Unable to broadcast {} leaving: {}.", uuid, why);
                }
            }
        }

        // Send the heartbeat to all clients.
        if let Err(why) = self.broadcast(ping_packet, None).await {
            sprintln!("ERROR sending heartbeat: {}", why)
        }
    }

    /// Sends a packet to the client with the uuid.
    pub async fn send_packet_to_uuid(
        &self,
        uuid: &Uuid,
        packet: Packet,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(client) = self.client_cache.get(uuid).await {
            Self::exec_send(&self.socket, &client.addr, packet)
                .await
                .map(|_| ())
        } else {
            Err("unable to find client".into())
        }
    }

    /// Sends a packet to the client with the uuid.
    pub async fn exec_send(
        socket: &UdpSocket,
        addr: &SocketAddr,
        packet: Packet,
    ) -> Result<usize, Box<dyn Error>> {
        let sent_bytes = socket.send_to(&packet.to_bytes(), addr).await?;
        Ok(sent_bytes)
    }

    /// Broadcasts a packet to multiple clients.
    /// If filter is None, broadcast to all clients in `cache`.
    /// If filter is Some and not empty, broadcast to only UUIDs in `cache`.
    /// If filter is Some and empty, broadcast to nobody.
    pub async fn broadcast(
        &self,
        packet: Packet,
        filter: Option<HashSet<Uuid>>,
    ) -> Result<(), Box<dyn Error>> {
        Self::exec_broadcast(&self.socket, &self.client_cache, packet, filter).await
    }

    /// Broadcasts a packet to multiple clients.
    /// If filter is None, broadcast to all clients in `cache`.
    /// If filter is Some and not empty, broadcast to only UUIDs in `cache`.
    /// If filter is Some and empty, broadcast to nobody.
    async fn exec_broadcast(
        socket: &UdpSocket,
        cache: &ClientCache,
        packet: Packet,
        filter: Option<HashSet<Uuid>>,
    ) -> Result<(), Box<dyn Error>> {
        // Get the clients to send to.
        let clients = {
            match filter {
                None => cache.values().await,
                Some(uuids) if !uuids.is_empty() => cache
                    .values()
                    .await
                    .into_iter()
                    .filter(|client| uuids.contains(&client.uuid))
                    .collect::<Vec<Client>>(),
                _ => Vec::new(),
            }
        };

        // Broadcast to all selected clients.
        for client in clients {
            let _ = Self::exec_send(socket, &client.addr, packet.clone()).await;
        }

        Ok(())
    }

    /// Sends a packet configuration to clients.
    async fn send_configuration(&self, packet_config: PacketConfiguration) {
        let response = match packet_config {
            PacketConfiguration::Empty => Ok(()),
            PacketConfiguration::Single(packet) => {
                self.send_packet_to_uuid(&packet.uuid(), packet).await
            }
            PacketConfiguration::Broadcast(packet, _scope) => {
                // NOTE: Currently assuming GLOBAL scope for broadcast.
                let c: HashSet<Uuid> = self.client_cache.keys().await;
                Self::exec_broadcast(&self.socket, &self.client_cache, packet, Some(c)).await
            }
            PacketConfiguration::SuccessBroadcast(to_client, to_broadcast, scope) => {
                // NOTE: Currently assuming GLOBAL scope for broadcast.
                if let Err(why) = self.send_packet_to_uuid(&to_client.uuid(), to_client).await {
                    sprintln!("ERROR WRITING {}", why);
                }

                let clients: HashSet<Uuid> = match scope {
                    BroadcastScope::Local(uuids) => uuids,
                    BroadcastScope::Global => self.client_cache.keys().await,
                };
                Self::exec_broadcast(
                    &self.socket,
                    &self.client_cache,
                    to_broadcast,
                    Some(clients),
                )
                .await
            }
        };

        if let Err(why) = response {
            sprintln!("FORWARD {}", why);
        }
    }
}
