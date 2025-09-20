use crate::{customer_connection::CustomerConnection, receiver::TcpReceiver};
use actix::{Addr, Message};
use app_utils::payment_type::PaymentType;
use serde::Serialize;
use server::apps_info::delivery_info::DeliveryInfo;

#[derive(Message, Serialize, Debug)]
#[rtype(result = "()")]
pub struct HandshakeReceived;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct SetReceiver {
    /// Comunicacion con el servidor. Direccion del actor TcpReceiver.
    pub receiver: Addr<TcpReceiver>,
}

///Mensaje que le envia el restaurante al servidor para que almacene su informacion para cuando un cliente la solicite.
#[derive(Message, Serialize, Debug)]
#[rtype(result = "()")]
pub struct RegisterRestaurant {
    pub socket: String,
    pub name: String,
    pub position: (u64, u64),
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct GetDeliveries;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct NearbyDeliveries {
    // Informacion de los deliveries
    pub nearby_deliveries: Vec<DeliveryInfo>,
}

#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
pub struct RequestDelivery {
    pub id: usize,
    pub origin: (u64, u64),
    pub destination: (u64, u64),
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct NewCustomerConnection {
    pub addr: Addr<CustomerConnection>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct NewOrder {
    pub total_price: f64,
    pub client_conn: Addr<CustomerConnection>,
    pub customer_position: (u64, u64),
    pub payment_type: PaymentType,
    pub customer_socket: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AcceptDelivery {
    pub order_id: u64,
    pub delivery_socket: String,
    pub delivery_position: (u64, u64),
    pub delivery_name: String,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct RestaurantOrderResponse {
    pub accepted: bool,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct OrderConfirmed {
    pub order_id: u64,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct PrepareOrder {
    pub order_id: u64,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct OrderReadyToPickup {
    pub order_id: u64,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RecoverOrderData {
    pub order_id: u64,
    pub addr: Addr<CustomerConnection>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TryNextDelivery;

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct RecoverDelivery {
    pub order_id: u64,
}
