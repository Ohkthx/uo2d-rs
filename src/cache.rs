use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use uuid::Uuid;

use crate::{packet::Packet, server::Client};

#[derive(Clone)]
pub struct PacketCache(Arc<Mutex<Vec<Packet>>>);

impl PacketCache {
    /// Creates a new PacketCache with an empty packet list.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    /// Retrieve received packets from the cache.
    pub fn get_all(&self) -> Vec<Packet> {
        let mut received = self.0.lock().unwrap();
        std::mem::take(&mut *received)
    }

    /// Add a new packet to the cache.
    pub fn add(&self, packet: Packet) {
        self.0.lock().unwrap().push(packet)
    }
}

#[derive(Clone)]
pub struct ClientCache(Arc<Mutex<HashMap<Uuid, Client>>>);

impl ClientCache {
    /// Creates a new ClientCache with an empty client map.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    /// Exposes the lock to the caller, allowing direct mutable access to the cache.
    /// The caller is responsible for handling the lock correctly.
    pub fn lock(&self) -> MutexGuard<HashMap<Uuid, Client>> {
        self.0.lock().unwrap()
    }

    /// Retrieve a client from the cache.
    pub fn get(&self, uuid: &Uuid) -> Option<Client> {
        self.lock().get(uuid).cloned()
    }

    /// Add a new client to the cache.
    pub fn add(&self, client: Client) {
        self.lock().insert(client.uuid, client);
    }

    /// Retrieves a vector of clients from the cache.
    pub fn values(&self) -> Vec<Client> {
        self.lock().values().cloned().collect()
    }

    pub fn keys(&self) -> Vec<Uuid> {
        self.lock().keys().cloned().collect()
    }

    pub fn remove(&self, uuid: &Uuid) -> Option<Client> {
        self.lock().remove(uuid)
    }
}
