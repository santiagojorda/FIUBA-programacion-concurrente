use actix::Message;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
/// Internal message to handle a ping to the coordinator
pub struct PingCoordinator;

#[derive(Debug, Clone, Message)]
#[rtype(result = "bool")]
/// This message allows an admin to check if it is the coordinator
pub struct AmICoordinator;

#[derive(Debug, Clone, Message)]
#[rtype(result = "Option<SocketAddr>")]
/// This message allows an admin to get the coordinator's address
pub struct GetCoordAddr;

#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
/// This message allows an admin to start an election
pub struct StartElection;

#[derive(Debug, Serialize, Deserialize, Clone, Message)]
#[rtype(result = "()")]
/// This message can be sent and received to indicate an election is happening
pub struct ElectionMessage {
    pub candidates: Vec<SocketAddr>,
}

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
/// This message is received by the coordinator and responds with an Ack
pub struct PingMessage {
    pub sender_id: SocketAddr,
}

#[derive(Debug, Serialize, Deserialize, Clone, Message)]
#[rtype(result = "()")]
/// Broadcast message to notify who is the new coordinator
pub struct CoordinatorMessage {
    pub coordinator: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
/// Internal message to set the current coordinator
pub struct SetCoordId {
    pub coord_id: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "Option<SocketAddr>")]
/// Internal message to get the current coordinator
pub struct GetCoordId;
