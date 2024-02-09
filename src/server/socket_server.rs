use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, timeout};
use uuid::Uuid;

use crate::packet::payloads::{MessagePayload, PingPayload};
use crate::packet::{Action, Packet, PacketConfiguration, Payload};
use crate::server::packet_processor::process_packet;
use crate::server::Client;
use crate::sprintln;
use crate::util::get_now;

/// Used to share clients.
type ClientsMap = Arc<Mutex<HashMap<Uuid, Client>>>;

const HEARTBEAT_INTERVAL: u64 = 5;
const MAX_HEARTBEAT_INTERVAL: u64 = HEARTBEAT_INTERVAL * 3;

/// Server instance responsible for managing clients and send/recving updates.
pub struct SocketServer {
    /// Current active clients.
    clients: ClientsMap,
}

impl SocketServer {
    /// Create a new instance of the srever.
    fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Starts the server for listening for incoming connections.
    pub fn start(address: &str, block: bool) -> Result<(), Box<dyn Error>> {
        let address = address.to_string();

        let rt = runtime::Runtime::new()?;
        if block {
            // Use `block_on` to block the current thread until the future completes.
            rt.block_on(async move {
                let mut server = Self::new();
                if let Err(why) = server.async_main(address).await {
                    eprintln!("ERROR: {}", why);
                };
            });
        } else {
            // For non-blocking behavior, spawn the future without waiting for it to complete.
            rt.spawn(async move {
                let mut server = Self::new();
                if let Err(why) = server.async_main(address).await {
                    eprintln!("ERROR: {}", why);
                };
            });
        }

        Ok(())
    }

    async fn async_main(&mut self, address: String) -> Result<(), Box<dyn Error>> {
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
                // Shutdown signal received.
                _ = shutdown_signals => {
                    sprintln!("Shutting down.");
                    let packet = Packet::new(
                        Action::Shutdown,
                        Uuid::nil(),
                        Payload::Message(MessagePayload::new("Server is shutting down.")),
                    );

                    self.broadcast(packet, None).await?;
                    sleep(Duration::from_secs(1)).await;
                    break;
                },
            }
        }

        Ok(())
    }

    /// Sends a packet for the clients to respond to, ensures they are still alive.
    async fn send_heartbeat(&mut self) -> Result<(), Box<dyn Error>> {
        // UUID they must respond with.
        let ping_id = Uuid::new_v4();

        let ping_packet = Packet::new(
            Action::Ping,
            Uuid::new_v4(),
            Payload::Ping(PingPayload::new(ping_id)),
        );

        // Update and clean the clients.
        {
            let mut expired: HashSet<Uuid> = HashSet::new();
            let mut clients = self.clients.lock().unwrap();
            let now = get_now();

            for (_, client) in clients.iter_mut() {
                if now - client.last_ping > MAX_HEARTBEAT_INTERVAL {
                    expired.insert(client.uuid);
                } else {
                    client.ping_id = ping_id;
                }
            }

            // Remove the expired clients.
            for uuid in expired {
                sprintln!("EXPIRED SESSION: {}", uuid);
                clients.remove(&uuid);
            }
        }

        // Send the heartbeat to all clients.
        self.broadcast(ping_packet, None).await?;
        Ok(())
    }

    /// Broadcasts a packet to multiple clients.
    /// If filter is None, broadcast to all clients in `clients_map`.
    /// If filter is Some and not empty, broadcast to only UUIDs in `clients_map`.
    /// If filter is Some and empty, broadcast to nobody.
    pub async fn broadcast(
        &mut self,
        packet: Packet,
        filter: Option<&[Uuid]>,
    ) -> Result<(), Box<dyn Error>> {
        Self::exec_broadcast(&mut self.clients, packet, filter).await
    }

    /// Broadcasts a packet to multiple clients.
    /// If filter is None, broadcast to all clients in `clients_map`.
    /// If filter is Some and not empty, broadcast to only UUIDs in `clients_map`.
    /// If filter is Some and empty, broadcast to nobody.
    async fn exec_broadcast(
        clients_map: &mut ClientsMap,
        packet: Packet,
        filter: Option<&[Uuid]>,
    ) -> Result<(), Box<dyn Error>> {
        let packet_bytes = packet.to_bytes();

        // Get the clients to send to.
        let clients = {
            let lock = clients_map.lock().unwrap();
            match filter {
                None => lock
                    .iter()
                    .map(|(_addr, tx)| tx.clone())
                    .collect::<Vec<_>>(),
                Some(uuids) if !uuids.is_empty() => lock
                    .iter()
                    .filter(|(id, _)| uuids.contains(id))
                    .map(|(_addr, tx)| tx.clone())
                    .collect::<Vec<_>>(),
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
    async fn listen(&mut self, mut socket: TcpStream, addr: SocketAddr) {
        // Channels for send/recving meessages from client.
        let (ctx, mut crx) = mpsc::channel::<Vec<u8>>(100);

        // Channels for send/recving meessages from handler.
        let (mut htx, mut hrx) = mpsc::channel::<Vec<u8>>(100);

        // Assign UUID to the new client.
        let uuid = Uuid::new_v4();
        sprintln!("{} has joined.", uuid);
        {
            // Store the sender in the clients map
            let mut clients = self.clients.lock().unwrap();
            clients.insert(uuid, Client::new(uuid, addr, ctx));
        };

        // Start packet handler.
        let mut buf = vec![0; 1024];
        let mut all_clients = self.clients.clone();
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
                        match process_packet(&mut htx, uuid, packet).await {
                            PacketConfiguration::Empty => (),
                            PacketConfiguration::Single(packet) => {
                                if let Err(why) = socket.write_all(&packet.to_bytes()).await {
                                    sprintln!("ERROR WRITING {}", why);
                                }
                            }
                            PacketConfiguration::Broadcast(packet, _scope) => {
                                // NOTE: Currently assuming GLOBAL scope for broadcast.
                                let c: Vec<Uuid> = all_clients.lock().unwrap().keys().cloned().collect();
                                end_session = packet.action() == Action::ClientLeave;
                                if let Err(why) = Self::exec_broadcast(&mut all_clients, packet, Some(&c)).await {
                                    sprintln!("ERROR BROADCAST {}", why);
                                }
                            }
                            PacketConfiguration::SuccessBroadcast(to_client, to_broadcast, _scope) => {
                                // NOTE: Currently assuming GLOBAL scope for broadcast.
                                let c: Vec<Uuid> = all_clients.lock().unwrap().keys().cloned().filter(|u| *u != uuid).collect();
                                if let Err(why) = socket.write_all(&to_client.to_bytes()).await {
                                    sprintln!("ERROR WRITING {}", why);
                                }
                                if let Err(why) = Self::exec_broadcast(&mut all_clients, to_broadcast, Some(&c)).await {
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
                            if let Payload::Ping(ping) = packet.payload() {
                                if let Some(client) = all_clients.lock().unwrap().get_mut(&packet.uuid()) {
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
        self.clients.lock().unwrap().remove(&uuid);
    }
}
