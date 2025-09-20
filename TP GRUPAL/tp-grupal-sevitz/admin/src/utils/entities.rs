use crate::admin_actor::clients_to_admin::DriverStatus;
use actix::Addr;
use common::tcp_sender::TcpSender;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct PassengerEntity {
    pub passenger_position: (f32, f32),
    pub passenger_destination: (f32, f32),
    pub passenger_sender: Option<Arc<Addr<TcpSender>>>,
}

#[derive(Debug, Clone)]
pub struct DriverEntity {
    pub driver_position: (f32, f32),
    pub current_passenger_id: Option<SocketAddr>,
    pub driver_sender: Option<Arc<Addr<TcpSender>>>,
    pub status: DriverStatus,
    pub time_stamp: Instant,
}
