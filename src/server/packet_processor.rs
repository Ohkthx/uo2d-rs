use tokio::sync::mpsc;
use uuid::Uuid;

use crate::packet::*;

/// Sends data from handler to server.
async fn fwd_packet(tx: &mpsc::Sender<Vec<u8>>, packet: Packet) {
    let tx = tx.clone();
    tokio::spawn(async move {
        let _ = tx.send(packet.to_bytes()).await;
    });
}

/// Processes all packet types.
pub(crate) async fn process_packet(
    tx: &mut mpsc::Sender<Vec<u8>>,
    uuid: Uuid,
    packet: Packet,
) -> PacketConfiguration {
    let _puuid = packet.uuid();
    let payload = packet.payload();
    match packet.action() {
        Action::Ping => ping(tx, uuid, payload).await,
        Action::Message => message(uuid, payload),
        Action::ClientJoin => client_join(uuid, payload),
        Action::ClientLeave => client_leave(uuid),
        Action::Movement => movement(uuid, payload),
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

fn client_join(uuid: Uuid, payload: Payload) -> PacketConfiguration {
    let payload = match payload {
        Payload::Movement(data) => data,
        _ => return PacketConfiguration::Empty,
    };

    let to_client = Packet::new(Action::Success, uuid, Payload::Empty);
    let to_broadcast = Packet::new(Action::ClientJoin, uuid, Payload::Movement(payload));
    PacketConfiguration::SuccessBroadcast(to_client, to_broadcast, BroadcastScope::Global)
}

fn client_leave(uuid: Uuid) -> PacketConfiguration {
    let packet = Packet::new(Action::ClientLeave, uuid, Payload::Empty);
    PacketConfiguration::Broadcast(packet, BroadcastScope::Global)
}

fn movement(uuid: Uuid, payload: Payload) -> PacketConfiguration {
    let payload = match payload {
        Payload::Movement(data) => data,
        _ => return PacketConfiguration::Empty,
    };

    let packet = Packet::new(Action::Movement, uuid, Payload::Movement(payload));
    PacketConfiguration::Broadcast(packet, BroadcastScope::Local)
}
