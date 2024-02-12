use uuid::Uuid;

use crate::{cprintln, packet::*};

use super::gamestate::Gamestate;
use super::socket_client::SocketClient;

/// Processes all packet types.
pub(crate) fn processor(
    client: &mut SocketClient,
    gamestate: &mut Gamestate,
    packet: Packet,
) -> Option<(Action, Payload)> {
    let puuid = packet.uuid();
    let payload = packet.payload();
    match packet.action() {
        Action::Ping => ping(payload),
        Action::Success => success(client, puuid),
        Action::Shutdown => shutdown(gamestate),
        Action::Message => message(puuid, payload),
        Action::ClientJoin => client_join(gamestate, puuid, payload),
        Action::ClientLeave => client_leave(gamestate, puuid),
        Action::Movement => movement(gamestate, puuid, payload),
        _ => None,
    }
}

fn ping(payload: Payload) -> Option<(Action, Payload)> {
    let payload = match payload {
        Payload::Ping(data) => data,
        _ => return None,
    };

    Some((Action::Ping, Payload::Ping(payload)))
}

fn success(client: &mut SocketClient, uuid: Uuid) -> Option<(Action, Payload)> {
    client.uuid = uuid;
    None
}

fn shutdown(gamestate: &mut Gamestate) -> Option<(Action, Payload)> {
    gamestate.kill = true;
    cprintln!("Server is shutting down.");
    None
}

fn message(uuid: Uuid, payload: Payload) -> Option<(Action, Payload)> {
    let payload = match payload {
        Payload::Message(data) => data,
        _ => return None,
    };

    cprintln!("{}: {}", uuid, payload.message);
    None
}

fn client_join(
    gamestate: &mut Gamestate,
    uuid: Uuid,
    payload: Payload,
) -> Option<(Action, Payload)> {
    let payload = match payload {
        Payload::Movement(data) => data,
        _ => return None,
    };

    cprintln!("{} has joined.", uuid);
    gamestate.upsert_entity(uuid, payload.position, payload.size);
    None
}

fn client_leave(gamestate: &mut Gamestate, uuid: Uuid) -> Option<(Action, Payload)> {
    cprintln!("{} has left.", uuid);
    gamestate.remove_entity(uuid);
    None
}

fn movement(gamestate: &mut Gamestate, uuid: Uuid, payload: Payload) -> Option<(Action, Payload)> {
    let payload = match payload {
        Payload::Movement(data) => data,
        _ => return None,
    };

    gamestate.upsert_entity(uuid, payload.position, payload.size);
    None
}
