use crate::admin_actor::clients_to_admin::DriverStatus;
use crate::utils::entities::{DriverEntity, PassengerEntity};
use actix::{Addr, Message};
use common::tcp_sender::TcpSender;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

pub struct DeadDriver {
    pub driver_id: SocketAddr,
    pub passenger_id: Option<SocketAddr>,
    pub passenger_sender: Option<Arc<Addr<TcpSender>>>,
}

#[derive(Message)]
#[rtype(result = "Option<PassengerEntity>")]
/// Message to get a passenger from the storage.
pub struct GetPassenger {
    pub id: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "Option<DriverEntity>")]
/// Message to get a driver from the storage.
pub struct GetDriver {
    pub id: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
/// Message to update a driver in the storage.
pub struct UpdateDriver {
    pub driver_id: SocketAddr,
    pub passenger_id: Option<SocketAddr>,
    pub status: DriverStatus,
    pub time_stamp: Instant,
}

#[derive(Message)]
#[rtype(result = "()")]
/// Message to insert a driver in the storage.
pub struct InsertDriver {
    pub id: SocketAddr,
    pub driver_position: (f32, f32),
    pub current_passenger_id: Option<SocketAddr>,
    pub driver_sender: Option<Arc<Addr<TcpSender>>>,
    pub status: DriverStatus,
    pub time_stamp: Instant,
}

#[derive(Message)]
#[rtype(result = "()")]
/// Message to insert a passenger in the storage.
pub struct InsertPassenger {
    pub id: SocketAddr,
    pub passenger_position: (f32, f32),
    pub passenger_destination: (f32, f32),
    pub passenger_sender: Option<Arc<Addr<TcpSender>>>,
}

#[derive(Message)]
#[rtype(result = "()")]
/// Message to remove a passenger from the storage.
pub struct RemovePassenger {
    pub id: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
/// Message to remove a driver from the storage.
pub struct RemoveDriver {
    pub id: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "Option<SocketAddr>")]
/// Message to get the nearest driver to a passenger.
pub struct GetNearestDriver;

#[derive(Message)]
#[rtype(result = "bool")]
/// Message to check if a passenger is finished.
pub struct IsFinished {
    pub passenger_id: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "Vec<DeadDriver>")]
/// Message to reap dead drivers.
pub struct ReapDeadDrivers;
