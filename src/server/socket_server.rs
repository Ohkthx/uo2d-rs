use std::collections::HashSet;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, timeout};
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
    /// Current active clients.
    client_cache: ClientCache,
    /// Cached packets for the gamestate.
    packet_cache: PacketCacheAsync,
}

impl SocketServer {
    fn new(packet_cache: PacketCacheAsync) -> Self {
        Self {
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
        let addr_clone = address.to_string();

        let rt = runtime::Runtime::new()?;
        // Use `block_on` to block the current thread until the future completes.
        rt.block_on(async move {
            let server = Self::new(cache);
            if let Err(why) = server.async_main(receiver, addr_clone).await {
                eprintln!("ERROR: {}", why);
            };
        });

        Ok(())
    }

    async fn async_main(
        &self,
        mut gs_rx: Receiver<PacketConfiguration>,
        address: String,
    ) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(address.clone())
            .await
            .expect("Failed to bind to address");
        sprintln!("Listening on {}", address);

        let mut ping_interval = interval(Duration::from_secs(HEARTBEAT_INTERVAL));

        loop {
            let shutdown_signals = async {
                let mut sigint =
                    signal(SignalKind::interrupt()).expect("Failed to bind SIGINT handler");
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("Failed to bind SIGTERM handler");

                tokio::select! {
                    _ = sigint.recv() => sprintln!("SIGINT received."),
                    _ = sigterm.recv() => sprintln!("SIGTERM received."),
                }
            };

            tokio::select! {
                _ = async {
                    while let Ok((socket, addr)) = listener.accept().await {
                        self.listen(socket, addr).await;
                    }
                } => {},
                // Sends the heartbeat to all clients.
                _ = ping_interval.tick() => {
                    self.send_heartbeat().await.expect("Failed to send heartbeat");
                },
                // Packet from the gamestate that needs to be sent out.
                packet = gs_rx.recv() => {
                    if let Some(config) = packet {
                        let response = match config {
                            PacketConfiguration::Empty => Ok(()),
                            PacketConfiguration::Single(packet) => self.send_packet(packet.uuid(), packet).await,
                            PacketConfiguration::Broadcast(packet, _scope) => self.broadcast(packet, None).await,
                            PacketConfiguration::SuccessBroadcast(to_client, to_all, scope) => {
                                if let Err(why) = self.send_packet(to_client.uuid(), to_client).await {
                                    sprintln!("ERROR WRITING {}", why);
                                }

                                let clients: HashSet<Uuid> = match scope {
                                    BroadcastScope::Local(uuids) => uuids,
                                    BroadcastScope::Global => {
                                        self.client_cache.keys().await.into_iter().filter(|u| *u != to_all.uuid()).collect()
                                    }
                                };
                                self.broadcast(to_all, Some(clients)).await
                            },
                        };

                        if let Err(why) = response {
                            sprintln!("FORWARD {}", why);
                        }
                    }
                },
                // Shutdown signal received.
                _ = shutdown_signals => {
                    sprintln!("Shutting down.");
                    let packet = Packet::new(
                        Action::Shutdown,
                        Uuid::nil(),
                        Payload::Message(MessagePayload::new("Server is shutting down.")),
                    );

                    self.packet_cache.add(packet.clone()).await;
                    self.broadcast(packet, None).await?;
                    sleep(Duration::from_secs(1)).await;
                    break;
                },
            }
        }

        Ok(())
    }

    /// Sends a packet for the clients to respond to, ensures they are still alive.
    async fn send_heartbeat(&self) -> Result<(), Box<dyn Error>> {
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
        self.broadcast(ping_packet, None).await?;
        Ok(())
    }

    /// Sends a packet to the client with the uuid.
    pub async fn send_packet(&self, uuid: Uuid, packet: Packet) -> Result<(), Box<dyn Error>> {
        let bytes = packet.to_bytes();
        let clients = self.client_cache.clone();

        // Spawn the async operation
        tokio::spawn(async move {
            // Lock and immediately drop to minimize lock holding time
            if let Some(client) = clients.get(&uuid).await {
                let _ = timeout(
                    Duration::from_secs(MAX_HEARTBEAT_INTERVAL),
                    client.tx.send(bytes),
                )
                .await;
            }
        });

        Ok(())
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
        Self::exec_broadcast(&self.client_cache, packet, filter).await
    }

    /// Broadcasts a packet to multiple clients.
    /// If filter is None, broadcast to all clients in `cache`.
    /// If filter is Some and not empty, broadcast to only UUIDs in `cache`.
    /// If filter is Some and empty, broadcast to nobody.
    async fn exec_broadcast(
        cache: &ClientCache,
        packet: Packet,
        filter: Option<HashSet<Uuid>>,
    ) -> Result<(), Box<dyn Error>> {
        let packet_bytes = packet.to_bytes();

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
        let _futures = clients
            .into_iter()
            .map(|client| {
                let packet_bytes = packet_bytes.clone();
                let tx = client.tx.clone();
                tokio::spawn(async move {
                    timeout(
                        Duration::from_secs(MAX_HEARTBEAT_INTERVAL),
                        tx.send(packet_bytes),
                    )
                    .await
                })
            })
            .collect::<Vec<_>>();

        // Note: Uncomment if want to wait for futures.
        // stream::iter(futures).for_each(|_| async {}).await;

        Ok(())
    }

    /// Listens for new connections.
    async fn listen(&self, mut socket: TcpStream, addr: SocketAddr) {
        // Channels for send/recving meessages from client.
        let (ctx, mut crx) = mpsc::channel::<Vec<u8>>(100);

        // Channels for send/recving meessages from handler.
        let (mut htx, mut hrx) = mpsc::channel::<Vec<u8>>(100);

        // Assign UUID to the new client.
        let uuid = Uuid::new_v4();
        sprintln!("{} has joined.", uuid);
        self.client_cache.add(Client::new(uuid, addr, ctx)).await;

        // Start packet handler.
        let mut buf = vec![0; 1024];
        let all_clients = self.client_cache.clone();
        let packet_cache = self.packet_cache.clone();
        let result: JoinHandle<()> = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Read a packet coming from client.
                    size = socket.read(&mut buf) => {
                        let n = match size {
                            Ok(0) => return,
                            Ok(n) => n,
                            Err(_) => return,
                        };

                        // Process the incoming packet from the client.
                        let packet = Packet::from_bytes(&buf[..n]);
                        let mut end_session: bool = false;
                        match process_packet(&packet_cache, &mut htx, uuid, packet).await {
                            PacketConfiguration::Empty => (),
                            PacketConfiguration::Single(packet) => {
                                if let Err(why) = socket.write_all(&packet.to_bytes()).await {
                                    sprintln!("ERROR WRITING {}", why);
                                }
                            }
                            PacketConfiguration::Broadcast(packet, _scope) => {
                                // NOTE: Currently assuming GLOBAL scope for broadcast.
                                let c: HashSet<Uuid> = all_clients.keys().await;
                                end_session = packet.action() == Action::ClientLeave;
                                if let Err(why) = Self::exec_broadcast(&all_clients, packet, Some(c)).await {
                                    sprintln!("ERROR BROADCAST {}", why);
                                }
                            }
                            PacketConfiguration::SuccessBroadcast(to_client, to_broadcast, scope) => {
                                // NOTE: Currently assuming GLOBAL scope for broadcast.
                                if let Err(why) = socket.write_all(&to_client.to_bytes()).await {
                                    sprintln!("ERROR WRITING {}", why);
                                }

                                let clients: HashSet<Uuid> = match scope {
                                    BroadcastScope::Local(uuids) => uuids,
                                    BroadcastScope::Global => {
                                        all_clients.keys().await.into_iter().filter(|u| *u != uuid).collect()
                                    }
                                };
                                if let Err(why) = Self::exec_broadcast(&all_clients, to_broadcast, Some(clients)).await {
                                    sprintln!("ERROR BROADCAST {}", why);
                                }
                            }
                        }

                        if end_session {
                            return;
                        }
                    }
                    // Broadcasted message that needs to be sent.
                    message = crx.recv() => {
                        if let Some(msg) = message {
                            if let Err(why) = socket.write_all(&msg).await {
                                sprintln!("ERROR WRITING {}", why);
                            }
                        }
                    },
                    // Message from the packet processor.
                    handler_message = hrx.recv() => {
                        if let Some(msg) = handler_message {
                            let packet: Packet = Packet::from_bytes(&msg) ;
                            if let Payload::Uuid(ping) = packet.payload() {
                                if let Some(client) = all_clients.lock().await.get_mut(&packet.uuid()) {
                                    if client.ping_id == ping.uuid {
                                        client.last_ping = get_now();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        // Remove the client from being tracked.
        if result.await.is_err() {
            sprintln!("Problem with {} exiting.", uuid);
        }

        sprintln!("{} has left.", uuid);
        self.client_cache.remove(&uuid).await;
    }
}
