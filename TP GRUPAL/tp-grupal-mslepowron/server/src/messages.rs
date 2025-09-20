use crate::apps_info::delivery_info::DeliveryInfo;
use crate::apps_info::restaurant_info::RestaurantData;
use crate::server_connection::ServerConnection;
use crate::tcp_sender::TcpSender;
use actix::Addr;
use actix::Message;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
pub struct UpdateNetworkState {
    /// Contador de ids
    pub ids_count: u64,
    /// Un hashmap con los conductores disponibles.
    pub deliveries: HashMap<String, DeliveryInfo>,
    /// Un hashmap que mapea el nombre del pasajero a un id
    pub customers: HashMap<String, u64>,
    pub restaurants: HashMap<String, RestaurantData>,
    /// Sockets UDP de servidores replica segun el id
    pub udp_sockets_replicas: HashMap<u64, String>,
    /// Vector con los ids de los servidores que estan desconectados
    pub disconnected_servers: Vec<u64>,
}

/// Mensaje enviado al servidor cuando se recibe una nueva conexión.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct NewServerConnection {
    /// Address del actor TcpSender
    pub addr: Addr<TcpSender>,
    /// Socket de la conexión (IP y puerto).
    pub socket: String,
    /// Id de la conexión.
    pub id: u64,
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), String>")]
pub struct Ping {}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Pong {
    pub addr: Addr<TcpSender>,
}

/// Mensaje para indicar que se desató una elección de líder.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct StartElection {}

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct Election {
    /// ID del líder que se cayó.
    pub disconnected_leader_id: u64,
    /// Listado de IDs de servidores réplicas conectados
    pub server_ids: Vec<u64>,
    /// Número de secuencia del mensaje.
    pub sequence_number: u64,
}

#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct NewLeader {
    /// ID del server que determinó el nuevo líder.
    pub id_sender: u64,
    /// ID del nuevo líder.
    pub leader_id: u64,
    /// Número de secuencia.
    pub sequence_number: u64,
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct SetLeader {
    pub id_leader: u64,
    pub leader_connection: Addr<ServerConnection>,
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct CheckIfNeighborAcked {
    /// ID de la réplica a quien le envíe un mensaje
    pub neighbor_id: u64,
    /// Mensaje que envié
    pub serialized_msg: String,
    /// Numero de secuencia.
    pub sequence_number: u64,
}

/// Mensaje para esperar por un ACK de una réplica.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct WaitForNeighborAck {
    /// Mensaje que envíe a la réplica
    pub serialized_msg: String,
    /// ID de la réplica
    pub neighbor_id: u64,
    /// Numero de secuencia del mensaje
    pub sequence_number: u64,
}

/// Representa un ACK de una réplica vecina.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct NeighborAck {
    /// ID de la réplica
    pub id: u64,
    /// Mensaje al cual corresponde el ACK
    pub message: String,
    /// Numero de secuencia
    pub sequence_number: u64,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ConnectToServer {}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Stop {}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct SendLeaderId {
    pub addr: String,
}

/// Mensaje cuando se conecta a la aplicacion, anunciando sus datos y posicion.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct CustomerConnected {
    /// Nombre del customer
    pub name: String,
    /// Address del actor del TcpSender del customer
    pub addr: Addr<TcpSender>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct DeliveryWorkerConnected {
    pub name: String,
    pub addr: Addr<TcpSender>,
}

///Mensaje recibido desde el tcp sender del Server y enviado al actor Server para registrar al restaurante.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct NewRestaurant {
    pub socket: String,
    pub name: String,
    pub position: (u64, u64),
}

#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub struct UpdateRestaurantSocket {
    pub socket: String,
    pub name: String,
}

///Mensaje recibido desde el tcp sender del Server para obtener informacion de los restaurantes disponibles.
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct GetRestaurants {
    pub _customer_id: u64,
    pub position: (u64, u64),
    pub addr: Addr<TcpSender>,
}

#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NearbyRestaurants {
    pub nearby_restaurants: Vec<RestaurantData>,
}

/// El repartidor se registra mandando su posición y socket
#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct RegisterDelivery {
    pub position: (u64, u64),
    pub socket: String,
    pub sender_addr: Addr<TcpSender>,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct GetDeliveries {
    pub id: u64,
    pub position: (u64, u64),
    pub addr: Addr<TcpSender>,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct NearbyDeliveries {
    pub nearby_deliveries: Vec<DeliveryInfo>,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct DeliveryStatus {
    pub customer_id: u64,
    pub message: String,
}

#[derive(Message, Serialize, Debug)]
#[rtype(result = "()")]
pub struct RequestDelivery {
    pub delivery_id: u64,
    pub order_id: u64,
    pub restaurant_position: (u64, u64),
    pub customer_position: (u64, u64),
}

#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
pub struct BusyDeliveryWorker {
    pub id: u64,
}

#[derive(Message, Debug, Serialize)]
#[rtype(result = "()")]
pub struct FreeDeliveryWorker {
    pub id: u64,
    pub position: (u64, u64),
    pub socket: String,
}
