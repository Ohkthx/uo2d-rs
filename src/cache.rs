use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex as SyncMutex};

use tokio::sync::{Mutex as AsyncMutex, MutexGuard};
use uuid::Uuid;

use crate::{packet::Packet, server::Client};

/// Holds packets and allows for access between threads.
#[derive(Clone)]
pub struct PacketCacheSync {
    /// Counts of each packet signature
    counts: Arc<SyncMutex<HashMap<Vec<u8>, usize>>>,
    packets: Arc<SyncMutex<Vec<Packet>>>,
    allowed_duplicates: usize,
}

impl PacketCacheSync {
    /// Creates a new cache for packets.
    pub fn new(allowed_duplicates: usize) -> Self {
        Self {
            counts: Arc::new(SyncMutex::new(HashMap::new())),
            packets: Arc::new(SyncMutex::new(Vec::new())),
            allowed_duplicates,
        }
    }

    /// Retrieve received packets from the cache. This clears the packet list and their counts.
    pub fn get_all(&self) -> Vec<Packet> {
        let mut counts = self.counts.lock().unwrap(); // Lock counts first
        let mut packets = self.packets.lock().unwrap(); // Then lock packets

        counts.clear();
        std::mem::take(&mut *packets)
    }

    /// Add a new packet to the cache if it doesn't exceed allowed duplicates.
    pub fn add(&self, packet: Packet) {
        let mut counts = self.counts.lock().unwrap(); // Lock counts first, consistent with get_all

        let signature = packet.signature();
        let count = counts.entry(signature.to_vec()).or_insert(0);

        if *count < self.allowed_duplicates {
            *count += 1;
            let mut packets = self.packets.lock().unwrap(); // Then lock packets
            packets.push(packet);
        }
    }
}

/// Holds packets and allows for access between threads.
#[derive(Clone)]
pub struct PacketCacheAsync {
    /// Counts of each packet signature
    counts: Arc<AsyncMutex<HashMap<Vec<u8>, usize>>>,
    packets: Arc<AsyncMutex<Vec<Packet>>>,
    allowed_duplicates: usize,
}

impl PacketCacheAsync {
    /// Creates a new cache for packets.
    pub fn new(allowed_duplicates: usize) -> Self {
        Self {
            counts: Arc::new(AsyncMutex::new(HashMap::new())),
            packets: Arc::new(AsyncMutex::new(Vec::new())),
            allowed_duplicates,
        }
    }

    /// Retrieve received packets from the cache. This clears the packet list and their counts.
    pub async fn get_all(&self) -> Vec<Packet> {
        let mut counts = self.counts.lock().await;
        let mut packets = self.packets.lock().await;

        counts.clear();
        std::mem::take(&mut *packets)
    }

    /// Add a new packet to the cache if it doesn't exceed allowed duplicates.
    pub async fn add(&self, packet: Packet) {
        let mut counts = self.counts.lock().await;

        let signature = packet.signature();
        let count = counts.entry(signature.to_vec()).or_insert(0);

        if *count <= self.allowed_duplicates {
            *count += 1;

            let mut packets = self.packets.lock().await;
            packets.push(packet);
        }
    }
}

/// Holds all clients that are current connected.
#[derive(Clone)]
pub struct ClientCache {
    clients: Arc<AsyncMutex<HashMap<Uuid, Client>>>,
    addr: Arc<AsyncMutex<HashMap<SocketAddr, Uuid>>>,
}

impl ClientCache {
    /// Creates a new ClientCache with an empty client map.
    pub fn new() -> Self {
        Self {
            clients: Arc::new(AsyncMutex::new(HashMap::new())),
            addr: Arc::new(AsyncMutex::new(HashMap::new())),
        }
    }

    /// Exposes the lock to the caller, allowing direct mutable access to the cache.
    /// The caller is responsible for handling the lock correctly.
    pub async fn lock(&self) -> MutexGuard<HashMap<Uuid, Client>> {
        self.clients.lock().await
    }

    /// Retrieve a client from the cache.
    pub async fn get(&self, uuid: &Uuid) -> Option<Client> {
        self.lock().await.get(uuid).cloned()
    }

    /// Retrieve a UUID attached to an address from the cache.
    pub async fn get_uuid(&self, addr: &SocketAddr) -> Option<Uuid> {
        self.addr.lock().await.get(addr).cloned()
    }

    /// Add a new client to the cache.
    pub async fn add(&self, client: Client) {
        self.addr.lock().await.insert(client.addr, client.uuid);
        self.lock().await.insert(client.uuid, client);
    }

    /// Retrieves a vector of clients from the cache.
    pub async fn values(&self) -> Vec<Client> {
        self.lock().await.values().cloned().collect()
    }

    pub async fn keys(&self) -> HashSet<Uuid> {
        self.lock().await.keys().cloned().collect()
    }

    pub async fn remove(&self, uuid: &Uuid) -> Option<Client> {
        let client = self.get(uuid).await;
        if let Some(client) = client {
            self.addr.lock().await.remove(&client.addr);
        }
        self.lock().await.remove(uuid)
    }
}
