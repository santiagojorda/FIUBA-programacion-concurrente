use crate::{receiver::TcpReceiver, restaurant_connection::RestaurantConnection};
use actix::{Addr, Message};
use serde::Serialize;

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct HandshakeReceived;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct NewConnection {
    pub addr: Addr<RestaurantConnection>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SetReceiver {
    /// Actor TcpReceiver para comunicarse con el servidor
    pub receiver: Addr<TcpReceiver>,
}

#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
pub struct LoginDelivery {
    pub name: String,
    pub position: (u64, u64),
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct SaveLoginDeliveryInfo {
    pub id: u64,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RegisterDelivery {
    pub position: (u64, u64),
    pub delivery_socket: String,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RequestDelivery {
    pub id_order: u64,
    pub restaurant_position: (u64, u64),
    pub customer_position: (u64, u64),
    pub customer_socket: String,
    pub addr: Addr<RestaurantConnection>,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct RestaurantConnectionMessage(pub String);

impl RestaurantConnectionMessage {
    pub fn new(message: String) -> Self {
        Self(format!("{message}\n"))
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct StartDelivery {}
