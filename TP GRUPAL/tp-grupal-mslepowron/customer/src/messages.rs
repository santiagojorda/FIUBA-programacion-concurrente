use crate::customer::Customer;
use crate::delivery_connection::DeliveryConnection;
use crate::receiver::TcpReceiver;
use actix::{Addr, Message};
use app_utils::payment_type::PaymentType;
use serde::Serialize;
use serde_json::Value;
use server::apps_info::restaurant_info::RestaurantData;

///Mensajes que un Customer envia a otros actores.

///Mensaje ACK para finalizar de entablar la comunicacion con el servidor
#[derive(Message, Serialize, Debug)]
#[rtype(result = "()")]
pub struct HandshakeReceived;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct SetReceiver {
    /// Comunicacion con el servidor. Direccion del actor TcpReceiver.
    pub receiver: Addr<TcpReceiver>,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
///Importa del archivo .json que guardo periodicamente 
/// los datos del customer los datos para volver a levantar al usuario
pub struct RecoverData {
    pub customer_status: Value,
}

#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
///Le envia al restaurante que lo vuelva a 
/// actualizar al customer en el estado de su orden 
/// porque el customer se cayo
pub struct RecoverCustomerOrder {
    pub order_id: u64,
}

///Mensaje que le envia el cliente al servidor para que registre su conexion Tcp como Customer.
#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
pub struct Login {
    /// Nombre del comensal
    pub name: String,
}

///Mensaje que le envia el cliente al servidor para que registre su conexion Tcp como Customer.
#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
pub struct SaveLoginInfo {
    /// Nombre del comensal
    pub id: u64,
}

#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
pub struct GetRestaurants {}

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct NearbyRestaurants {
    pub nearby_restaurants: Vec<RestaurantData>,
}

//MENSAJES AL GATEWAY

//mensaje para el actor gateway connection que se guarde la info
//de este custoemr
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct SetCustomer {
    pub customer: Addr<Customer>,
}

#[derive(Message, Serialize)]
#[rtype(result = "bool")]
pub struct PreparePayment {}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AuthPaymentResponse {
    pub accepted: bool,
}

#[derive(Message, Serialize)]
#[rtype(result = "bool")]
pub struct PrepareOrder {}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RestaurantOrderResponse {
    pub accepted: bool,
}

//Mensaje que indica que se continue con los flujos de pago y preparacion de pedido para el comensal
#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct CommitPayment {
    pub customer_id: u64,
    pub amount: f64,
}

#[derive(Message, Serialize)]
#[rtype(result = "bool")]
pub struct CommitOrder {
    pub customer_id: u64,
    pub payment_type: PaymentType,
    pub customer_position: (u64, u64),
    pub amount: f64,
    pub customer_socket: String,
}

//Mensaje que indica que se aborte con los flujos de pago y preparacion de pedido para el comensal
#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct Abort;

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct PaymentDenied;

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct PaymentConfirmed {
    pub amount: f64,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct OrderConfirmed {
    pub order_id: u64,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct OrderCanceled {}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct PreparingOrder {
    pub order_id: u64,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct OrderReadyToPickup {
    pub order_id: u64,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct ArrivedDestination {
    pub order_id: u64,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct OrderDelivered {
    pub order_id: u64,
}
#[derive(Message, Debug, Clone, Serialize)]
#[rtype(result = "()")]
pub struct DeliveryAccepted {
    /// lo manda el restaurante cuando encuentra un delivery
    pub order_id: u64,
    pub delivery_position: (u64, u64),
    pub delivery_socket: String,
    pub delivery_name: String,
}

#[derive(Message, Debug, Clone, Serialize)]
#[rtype(result = "()")]
pub struct StatusUpdate {
    pub message: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddDeliveryConnection {
    pub addr: Addr<DeliveryConnection>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RecoverDelivery {
    pub order_id: u64,
}
