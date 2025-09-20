use crate::payment_messages::PaymentMessageType;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// ---------------------------------- ADMIN MESSAGES ----------------------------------

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "(bool)")]
/// Message to send to Payment gateway
pub struct SendPaymentMessage {
    pub passenger_id: String,
    pub amount: f32,
    pub message_type: PaymentMessageType,
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
/// Confirmation from payment gateway
pub struct AuthConfirmation {
    pub passenger_id_ac: SocketAddr,
    pub is_authorized: bool,
}

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
/// Message response with the current coordinator address
pub struct WhoIsCoordinatorResponse {
    pub coord_id: SocketAddr,
}

//------------------------------------ CLIENT MESSAGES ----------------------------------

#[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
#[rtype(result = "()")]
/// Message to request a trip to the admins
pub struct RequestTrip {
    pub origin: (f32, f32),
    pub destination: (f32, f32),
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
/// Message to ask if driver can accept a trip
pub struct CanAcceptTrip {
    pub passenger_id_ca: SocketAddr,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
/// Response from driver to can accept trip request
pub struct CanAcceptTripResponse {
    pub passenger_id_car: SocketAddr,
    pub is_accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
/// Message to start a trip from admin to clients
pub struct StartTrip {
    pub passenger_id_st: SocketAddr,
    pub driver_id_st: SocketAddr,
    pub origin: (f32, f32),
    pub destination: (f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
/// Message to reject a trip from admin to passenger
pub struct RejectTrip {
    pub response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
/// Message to inform a trip finished from client to admin
pub struct FinishTrip {
    pub passenger_id_ft: SocketAddr,
    pub driver_id_ft: SocketAddr,
    pub destination_pos: (f32, f32),
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
/// Message to inform admin of driver location
pub struct DriverPosition {
    pub position: (f32, f32),
}
