use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{cache::PacketCacheAsync, packet::*};

/// Sends data from handler to server.
async fn fwd_packet(tx: &mpsc::Sender<Vec<u8>>, packet: Packet) {
    let tx = tx.clone();
    tokio::spawn(async move {
        let _ = tx.send(packet.to_bytes()).await;
    });
}

/// Processes all packet types.
pub(crate) async fn process_packet(
    packet_cache: &PacketCacheAsync,
    tx: &mut mpsc::Sender<Vec<u8>>,
    uuid: Uuid,
    packet: Packet,
) -> PacketConfiguration {
    let _puuid = packet.uuid();
    let payload = packet.payload();
    match packet.action() {
        Action::Ping => ping(tx, uuid, payload).await,
        Action::Message => message(uuid, payload),
        Action::ClientJoin => client_join(packet_cache, uuid, payload).await,
        Action::ClientLeave => client_leave(packet_cache, uuid).await,
        Action::Movement => movement(packet_cache, uuid, payload).await,
        _ => PacketConfiguration::Empty,
    }
}

async fn ping(tx: &mut mpsc::Sender<Vec<u8>>, uuid: Uuid, payload: Payload) -> PacketConfiguration {
    let payload = match payload {
        Payload::Ping(data) => data,
        _ => return PacketConfiguration::Empty,
    };

    let packet = Packet::new(Action::Ping, uuid, Payload::Ping(payload));
    fwd_packet(tx, packet).await;
    PacketConfiguration::Empty
}

fn message(uuid: Uuid, payload: Payload) -> PacketConfiguration {
    let payload = match payload {
        Payload::Message(data) => data,
        _ => return PacketConfiguration::Empty,
    };

    let packet = Packet::new(Action::Message, uuid, Payload::Message(payload));
    PacketConfiguration::Broadcast(packet, BroadcastScope::Global)
}

async fn client_join(
    packet_cache: &PacketCacheAsync,
    uuid: Uuid,
    payload: Payload,
) -> PacketConfiguration {
    let payload = match payload {
        Payload::Movement(data) => data,
        _ => return PacketConfiguration::Empty,
    };

    let to_client = Packet::new(Action::Success, uuid, Payload::Empty);
    let to_broadcast = Packet::new(Action::ClientJoin, uuid, Payload::Movement(payload));
    packet_cache.add(to_broadcast.clone()).await;
    PacketConfiguration::SuccessBroadcast(to_client, to_broadcast, BroadcastScope::Global)
}

async fn client_leave(packet_cache: &PacketCacheAsync, uuid: Uuid) -> PacketConfiguration {
    let packet = Packet::new(Action::ClientLeave, uuid, Payload::Empty);
    packet_cache.add(packet.clone()).await;
    PacketConfiguration::Broadcast(packet, BroadcastScope::Global)
}

async fn movement(
    packet_cache: &PacketCacheAsync,
    uuid: Uuid,
    payload: Payload,
) -> PacketConfiguration {
    let payload = match payload {
        Payload::Movement(data) => data,
        _ => return PacketConfiguration::Empty,
    };

    let packet = Packet::new(Action::Movement, uuid, Payload::Movement(payload));
    packet_cache.add(packet).await;
    PacketConfiguration::Empty
}
