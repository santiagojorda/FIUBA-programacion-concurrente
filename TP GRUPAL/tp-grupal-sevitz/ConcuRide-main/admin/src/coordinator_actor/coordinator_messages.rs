use crate::admin_actor::clients_to_admin::DriverStatus;
use actix::prelude::*;
use common::tcp_sender::TcpSender;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

pub type Peers = Arc<HashMap<SocketAddr, (Addr<TcpSender>, Instant)>>;

#[derive(Message)]
#[rtype(result = "()")]
/// This message connects the Coordinator to the Admins
/// to be able to send updates and requests
pub struct BecomeCoordinator;

#[derive(Message)]
#[rtype(result = "()")]
/// This message connects the Coordinator to a new peer
/// if it connected after the Coordinator was created
pub struct ConnectNewPeer {
    pub new_peer: SocketAddr,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
/// This enum represents the actions that can be done in updates
pub enum Action {
    Insert,
    Delete,
    Update,
}

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
#[rtype(result = "()")]
/// This message is used to update the passengers in the non coordinator Admins
pub struct UpdatePassengers {
    pub action: Action,
    pub passenger: SocketAddr,
    pub origin: (f32, f32),
    pub destination: (f32, f32),
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
/// This message is used to update the drivers in the non coordinator Admins
pub struct UpdateDrivers {
    pub action: Action,
    pub driver: SocketAddr,
    pub position: (f32, f32),
    pub current_passenger_id: Option<SocketAddr>,
    pub status: DriverStatus,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
/// This message is used to tell one of the admins to handle a trip for a passenger
pub struct HandleTrip {
    pub passenger_id_ht: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
/// Internal message to add a peer to the tcp handles dictionary
pub struct AddPeerToDict {
    pub peer_addr: SocketAddr,
    pub peer_sender: Addr<TcpSender>,
}

#[derive(Message)]
#[rtype(result = "Peers")]
/// Internal message to get the tcp handles dictionary
pub struct GetPeerDict;

#[derive(Message)]
#[rtype(result = "u8")]
/// Internal message to get the peer counter and increment it
/// its used for round robin load balancing
pub struct GetPeerCounter;
