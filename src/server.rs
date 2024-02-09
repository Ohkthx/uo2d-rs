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
use tokio::time::{interval, sleep, timeout};
use uuid::Uuid;

use crate::packet::payloads::{MessagePayload, PingPayload};
use crate::packet::{Action, Packet, Payload};
use crate::sprintln;
use crate::util::get_now;

const HEARTBEAT_INTERVAL: u64 = 5;
const MAX_HEARTBEAT_INTERVAL: u64 = HEARTBEAT_INTERVAL * 3;

/// Used to share clients.
type ClientsMap = Arc<Mutex<HashMap<Uuid, Client>>>;

/// Used to determine how a client quit/exit.
enum ClientQuit {
    Leave,
    Disconnect,
}

/// Holds all of the relevant client information for send/recving packets.
#[derive(Clone)]
struct Client {
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

/// Server instance responsible for managing clients and send/recving updates.
pub struct Server {
    /// Current active clients.
    clients: ClientsMap,
}

impl Server {
    /// Create a new instance of the srever.
    fn new() -> Server {
        Server {
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
                let mut server = Server::new();
                if let Err(why) = server.async_main(address).await {
                    eprintln!("ERROR: {}", why);
                };
            });
        } else {
            // For non-blocking behavior, spawn the future without waiting for it to complete.
            rt.spawn(async move {
                let mut server = Server::new();
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
        Server::exec_broadcast(&mut self.clients, packet, filter).await
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
                Some(uuids) if uuids.is_empty() => lock
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

    /// Sends data from handler to server.
    async fn from_handler(tx: &mpsc::Sender<Vec<u8>>, packet: Packet) {
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(packet.to_bytes()).await;
        });
    }

    /// Processes all packet types.
    async fn process_packet(
        tx: &mut mpsc::Sender<Vec<u8>>,
        uuid: Uuid,
        mut packet: Packet,
    ) -> Result<Option<Packet>, ClientQuit> {
        let (action, payload) = match packet.action() {
            Action::Ping => match packet.payload() {
                // Client needs to be updated to ensure it is not disconnected.
                Payload::Ping(_) => {
                    packet = packet.set_uuid(uuid); // Update the packet UUID to ensure client does not spoof.
                    Server::from_handler(tx, packet).await;
                    return Ok(None);
                }
                _ => return Ok(None),
            },
            Action::ClientJoin => (Action::Success, Payload::Empty),
            Action::ClientLeave => return Err(ClientQuit::Leave),
            _ => return Ok(None),
        };

        Ok(Some(Packet::new(action, uuid, payload)))
    }

    /// Listens for new connections.
    async fn listen(&mut self, mut socket: TcpStream, addr: SocketAddr) -> ClientQuit {
        // Channels for send/recving meessages from client.
        let (ctx, mut crx) = mpsc::channel::<Vec<u8>>(100);

        // Channels for send/recving meessages from handler.
        let (mut htx, mut hrx) = mpsc::channel::<Vec<u8>>(100);

        // Assing UUID to the new client.
        let uuid = Uuid::new_v4();
        let output = format!("{} has joined.", uuid);
        sprintln!("{}", output);
        let payload = Payload::Message(MessagePayload { message: output });

        // Broadcast client joining.
        let packet = Packet::new(Action::Message, Uuid::nil(), payload);
        let _ = self.broadcast(packet, None).await;

        {
            // Store the sender in the clients map
            let mut clients = self.clients.lock().unwrap();
            clients.insert(uuid, Client::new(uuid, addr, ctx));
        };

        // Start packet handler.
        let mut buf = vec![0; 1024];
        let mut clients_clone = self.clients.clone();
        let joiner = tokio::spawn(async move {
            let action = loop {
                tokio::select! {
                    // Read a packet coming from client.
                    size = socket.read(&mut buf) => {
                        let n = match size {
                            Ok(0) => return ClientQuit::Disconnect,
                            Ok(n) => n,
                            Err(_) => return ClientQuit::Disconnect,
                        };

                        let packet = Packet::from_bytes(&buf[..n]);

                        // Process the incoming packet from the client.
                        match Server::process_packet(&mut htx, uuid, packet).await {
                            Ok(Some(response)) => {
                                if let Err(why) = socket.write_all(&response.to_bytes()).await {
                                    sprintln!("ERROR WRITING {}", why);
                                }
                            },
                            Err(action) => break action,
                            _ => ()
                        }
                    },
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
                                if let Some(client) = clients_clone.lock().unwrap().get_mut(&packet.uuid()) {
                                    if client.ping_id == ping.uuid {
                                        client.last_ping = get_now();
                                    }
                                }
                            }
                        }
                    }
                }
            };

            // Client is no longer being processed, broadcast to all other clients.
            let client = clients_clone.lock().unwrap().remove(&uuid);
            if let Some(client) = client {
                let uuid = client.uuid;
                let message = match action {
                    ClientQuit::Disconnect => format!("{} has disconnected.", uuid),
                    ClientQuit::Leave => format!("{} has left.", uuid),
                };

                sprintln!("{}", message);
                let payload = Payload::Message(MessagePayload { message });

                let packet = Packet::new(Action::Message, Uuid::nil(), payload);
                let _ = Server::exec_broadcast(&mut clients_clone, packet, None).await;
            }

            action
        });

        joiner.await.expect("Unable to join server.")
    }
}
