use crate::apps_info::delivery_info::DeliveryInfo;
use crate::apps_info::restaurant_info::RestaurantData;
use crate::connection_listener::ConnectionListener;
use crate::messages::{
    BusyDeliveryWorker, CheckIfNeighborAcked, ConnectToServer, CustomerConnected, DeliveryStatus,
    DeliveryWorkerConnected, Election, FreeDeliveryWorker, GetDeliveries, GetRestaurants,
    NearbyDeliveries, NearbyRestaurants, NeighborAck, NewLeader, NewRestaurant,
    NewServerConnection, Ping, Pong, RegisterDelivery, RequestDelivery, SendLeaderId, SetLeader,
    StartElection, Stop, UpdateNetworkState, UpdateRestaurantSocket, WaitForNeighborAck,
};
use crate::server_connection::ServerConnection;
use crate::tcp_sender::{TcpMessage, TcpSender};
use crate::udp_receiver::Listen;
use crate::udp_sender::{UdpMessage, UdpSender};
use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, StreamHandler};
use actix_async_handler::async_handler;
use app_utils::constants::{MAX_DELIVERY_DISTANCE, MAX_RESTAURANT_DISTANCE, SERVER_UDP_PORT_RANGE};
use app_utils::utils::{distance_squared, id_to_tcp_address, serialize_message};
use colored::Colorize;
use log::{debug, error, info};
use serde_json::json;
use std::collections::HashMap;
use std::io;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader, split};
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_stream::wrappers::LinesStream;

/// Estructura del servidor
pub struct Server {
    pub id: u64,
    pub ids_count: u64,
    pub id_leader: u64,
    pub im_leader: bool,
    pub customers: HashMap<String, u64>,
    pub customers_address: HashMap<u64, Addr<TcpSender>>,
    ///contador de clientes conectados para saber que ids asignarles
    pub clients_connected: u64,
    ///Clientes de tipo DeliveryWorker con su ID
    pub delivery_workers: Vec<u64>,
    pub delivery_workers_info: Vec<DeliveryInfo>,
    ///Conexion TCP con cada DeliveryWorker, segun su ID
    pub delivery_workers_address: HashMap<u64, Addr<TcpSender>>,
    pub restaurants: HashMap<String, RestaurantData>,
    /// Conexion con el lider. Es None en caso de ser el lider.
    pub leader_connection: Option<Addr<ServerConnection>>,
    /// Listado de servers desconectados
    pub disconnected_servers: Vec<u64>,
    /// Booleano indicando si se recibio el ACK del vecino
    pub received_neighbor_ack: bool,
    /// Booleano indicando si hay una eleccion de lider en curso
    pub election_on_course: bool,
    /// ID del vecino actual
    pub actual_neighbor_id: u64,
    /// Numero de secuencia actual
    pub sequence_number: u64,
    /// Booleano que indica si se recibio ACK para un numero de secuencia
    pub message_received_ack: HashMap<u64, bool>,
    pub udp_sender: Option<Addr<UdpSender>>,
    pub udp_sockets_replicas: HashMap<u64, String>,
    pub servers: HashMap<u64, Addr<TcpSender>>,
}

impl Server {
    /// Actualiza el `actual_neighbor_id`, devuelve true o false dependiendo de si hay algun vecino conectado.
    fn update_neighbor(&mut self, neighbor_id: u64) -> bool {
        if !self.disconnected_servers.contains(&self.actual_neighbor_id) {
            return true;
        }
        self.actual_neighbor_id = get_neighbor_id(neighbor_id);
        let mut count = 0;
        loop {
            if count >= 5 {
                return false;
            }
            if !self.disconnected_servers.contains(&self.actual_neighbor_id)
                && self.actual_neighbor_id != self.id
            {
                break;
            }
            self.actual_neighbor_id = get_neighbor_id(self.actual_neighbor_id);
            count += 1;
        }
        true
    }
}

impl Actor for Server {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        debug!("Server::started");

        ctx.run_interval(Duration::from_millis(500), |act, ctx| {
            // Solo pingeo si no soy el lider ni estoy en eleccion
            if !act.im_leader && !act.election_on_course {
                ctx.address().do_send(Ping {});
            }
        });
    }
}

impl Handler<CustomerConnected> for Server {
    type Result = ();

    /// Maneja el mensaje de conexión de un cliente de tipo CUSTOMER, añadiéndolo al servidor si no existe o registrando su loggueo si es un cliente ya existente que se volvio a conectar.
    fn handle(&mut self, msg: CustomerConnected, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(id) = self.customers.get(&msg.name) {
            info!(
                "{}",
                format!("[SERVER] Volvio el comensal {} con ID {}.", msg.name, id).green()
            );

            self.customers_address.insert(*id, msg.addr.clone());
            // LE MANDO ALGO AL CUSTOMER: le envio su ID de sesion para que lo guarde
            let message = serialize_message("login_successful", json!({"id": id}));

            if let Err(e) = msg.addr.try_send(TcpMessage(format!("{}\n", message))) {
                error!("Error al enviar el mensaje: {}", e);
            }
        } else {
            self.clients_connected += 1;
            let id_customer = self.clients_connected;

            self.customers.insert(msg.name.clone(), id_customer);
            self.customers_address.insert(id_customer, msg.addr.clone());
            info!(
                "{}",
                format!(
                    "[SERVER] Bienvenido a la App, comensal {} con ID {}.",
                    msg.name, id_customer
                )
                .green()
            );
            // LE MANDO ALGO AL CUSTOMER
            let message = serialize_message("login_successful", json!({"id": id_customer}));

            if let Err(e) = msg.addr.try_send(TcpMessage(format!("{}\n", message))) {
                error!("Error al enviar el mensaje: {}", e);
            }
        }
    }
}

impl Handler<DeliveryWorkerConnected> for Server {
    type Result = ();

    fn handle(&mut self, msg: DeliveryWorkerConnected, _ctx: &mut Context<Self>) -> Self::Result {
        // 1) Asigna un nuevo ID
        self.clients_connected += 1;
        let id = self.clients_connected;

        // 2) Guarda en tu lista de delivery_workers
        self.delivery_workers.push(id);
        self.delivery_workers_address.insert(id, msg.addr.clone());

        info!("[SERVER] Repartidor {} conectado con ID {}.", msg.name, id);

        // 3) Envía el "login_successful_delivery" de vuelta
        let payload = json!({ "id": id });
        let m = serialize_message("login_successful_delivery", payload);
        if let Err(e) = msg.addr.try_send(TcpMessage(format!("{}\n", m))) {
            error!("[SERVER] Error al enviar login_successful_delivery: {}", e);
        }
    }
}

impl Handler<NewRestaurant> for Server {
    type Result = ();

    fn handle(&mut self, msg: NewRestaurant, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(restaurant) = self.restaurants.get(&msg.name) {
            info!(
                "{}",
                format!(
                    "El restaurante '{}' existe con ID {}\n",
                    msg.name, restaurant.id
                )
                .green()
            );
            if let Err(e) = _ctx.address().try_send(UpdateRestaurantSocket {
                //se updatea el socket xq que haya enviado un mensaje de newResturant pero ya estando en los datos del servidor indica una reconexion
                name: msg.name.clone(),
                socket: msg.socket.clone(),
            }) {
                error!("Error al enviar el mensaje: {}", e);
            }
            return;
        }

        self.clients_connected += 1;
        let id = self.clients_connected;

        let restaurant_data = RestaurantData {
            socket: msg.socket,
            position: msg.position,
            name: msg.name.clone(),
            id,
        };

        self.restaurants.insert(msg.name.clone(), restaurant_data); //si no existe creo el restaurante

        info!(
            "{}",
            format!("[SERVER]: Nuevo restaurante {} con ID {}", msg.name, id).blue()
        );
    }
}

impl Handler<GetRestaurants> for Server {
    type Result = ();

    /// envia al comensal una lsita de los restaurantes disponibles para pedir, si estan en un radio cercano al comensal.
    fn handle(&mut self, msg: GetRestaurants, _ctx: &mut Self::Context) -> Self::Result {
        // Filtrar restaurantes dentro del radio
        let nearby_restaurants: Vec<RestaurantData> = self
            .restaurants
            .values()
            .filter(|rst| {
                distance_squared(msg.position, rst.position)
                    <= MAX_RESTAURANT_DISTANCE * MAX_RESTAURANT_DISTANCE
            })
            .cloned()
            .collect();

        let restaurants_message = NearbyRestaurants { nearby_restaurants };

        let serialized_msg = serialize_message("nearby_restaurants", restaurants_message);
        if let Err(e) = msg.addr.try_send(TcpMessage::new(serialized_msg)) {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}

impl Handler<UpdateRestaurantSocket> for Server {
    type Result = ();

    /// Actualiza el socket del restaurante
    fn handle(&mut self, msg: UpdateRestaurantSocket, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::UpdateRestaurantSocket\n");

        debug!("Actualizando Socket restaurante: {}", msg.name.clone());

        if let Some(restaurant_info) = self.restaurants.get_mut(&msg.name.clone()) {
            debug!("Viejo Socket: {}", restaurant_info.socket.clone());
            debug!("Nuevo Socket: {}", msg.socket.clone());
            restaurant_info.socket = msg.socket.clone();
        }
    }
}

impl Handler<RegisterDelivery> for Server {
    type Result = ();

    fn handle(&mut self, msg: RegisterDelivery, _ctx: &mut Context<Self>) {
        // 1) asigna un nuevo ID
        self.clients_connected += 1;
        let id = self.clients_connected;

        // 2) almacena la info completa
        self.delivery_workers.push(id);
        self.delivery_workers_address
            .insert(id, msg.sender_addr.clone());
        self.delivery_workers_info.push(DeliveryInfo {
            id,
            position: msg.position,
            socket: msg.socket.clone(),
        });

        info!("[SERVER] Delivery #{} registrado en {:?}", id, msg.position);

        // 3) responde al propio repartidor con ID, posición y socket
        let payload = json!({
            "id":              id,
            "position":        msg.position,
            "delivery_socket": msg.socket,
        });
        let m = serialize_message("login_successful_delivery", payload);
        let _ = msg.sender_addr.try_send(TcpMessage::new(m));
    }
}

impl Handler<GetDeliveries> for Server {
    type Result = ();

    fn handle(&mut self, msg: GetDeliveries, _ctx: &mut Context<Self>) {
        let max_sq = MAX_DELIVERY_DISTANCE * MAX_DELIVERY_DISTANCE;
        let available = &self.delivery_workers_info;

        // 1) todos los que estén en rango
        let mut nearby: Vec<_> = available
            .iter()
            .filter(|info| distance_squared(info.position, msg.position) <= max_sq)
            .cloned()
            .collect();

        // 2) si ninguno en rango pero sí hay repartidores conectados, devolvemos todos ordenados
        if nearby.is_empty() && !available.is_empty() {
            info!("[SERVER] Ninguno en rango, reintentando con todos ordenados por distancia");
            let mut all: Vec<(u64, DeliveryInfo)> = available
                .iter()
                .map(|info| (distance_squared(info.position, msg.position), info.clone()))
                .collect();
            all.sort_by_key(|(d, _)| *d);
            nearby = all.into_iter().map(|(_, info)| info).collect();
        }

        info!(
            "[SERVER] NearbyDeliveries para restaurant #{} → {} repartidores",
            msg.id,
            nearby.len()
        );

        let resp = NearbyDeliveries {
            nearby_deliveries: nearby,
        };
        let _ = msg.addr.do_send(TcpMessage::new(serialize_message(
            "nearby_deliveries",
            resp,
        )));
    }
}

impl Handler<DeliveryStatus> for Server {
    type Result = ();

    fn handle(&mut self, msg: DeliveryStatus, _ctx: &mut Context<Self>) {
        // Busca al cliente en tu mapa customers_address
        if let Some(client_sender) = self.customers_address.get(&msg.customer_id) {
            // reframera como "status_update"
            let framed = serialize_message(
                "status_update",
                json!({
                    "message": msg.message,
                }),
            );
            let _ = client_sender.do_send(TcpMessage::new(framed));
        } else {
            error!("No encontré cliente con ID {}", msg.customer_id);
        }
    }
}

impl Handler<RequestDelivery> for Server {
    type Result = ();

    fn handle(&mut self, msg: RequestDelivery, _ctx: &mut Context<Self>) {
        if let Some(delivery_sender) = self.delivery_workers_address.get(&msg.delivery_id) {
            // reconstruyo exactamente el mismo request_delivery que vino
            let framed = serialize_message(
                "request_delivery",
                json!({
                    "order_id": msg.order_id,
                    "restaurant_position": msg.restaurant_position,
                    "customer_position": msg.customer_position,
                }),
            );
            let _ = delivery_sender.do_send(TcpMessage::new(framed));
        } else {
            error!("[SERVER] No encontré delivery #{}", msg.delivery_id);
        }
    }
}

impl Handler<BusyDeliveryWorker> for Server {
    type Result = ();

    fn handle(&mut self, msg: BusyDeliveryWorker, _ctx: &mut Context<Self>) -> Self::Result {
        // quitamos ese delivery de la lista de disponibles
        let before = self.delivery_workers_info.len();
        self.delivery_workers_info.retain(|d| d.id != msg.id);
        let after = self.delivery_workers_info.len();
        info!(
            "[SERVER] Delivery #{} ocupado -> removido de pool ({}→{} remain)",
            msg.id, before, after
        );
    }
}

impl Handler<FreeDeliveryWorker> for Server {
    type Result = ();

    fn handle(&mut self, msg: FreeDeliveryWorker, _ctx: &mut Context<Self>) {
        self.delivery_workers_info.push(DeliveryInfo {
            id: msg.id,
            position: msg.position,
            socket: msg.socket.clone(),
        });
        info!(
            "{}",
            format!("[SERVER] Delivery #{} libre → reingresado al pool", msg.id).magenta()
        );
    }
}

/// Devuelve IP y puerto UDP de un server segun el id
fn get_address(id: u64) -> String {
    let port = SERVER_UDP_PORT_RANGE + id;
    let address = format!("127.0.0.1:{}", port);
    address
}

/// Recibe el id de un server y devuelve el id del vecino de ese server.
pub fn get_neighbor_id(id: u64) -> u64 {
    if id == 5 { 1 } else { id + 1 }
}

impl Handler<NewServerConnection> for Server {
    type Result = ();

    /// Este mensaje se envia cuando una replica se conecta al servidor lider.
    /// Si la replica estaba desconectada, se la elimina de la lista de desconectados
    /// y se guarda la conexion.
    /// Tambien se envia el estado a todas las replicas.
    fn handle(&mut self, msg: NewServerConnection, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::NewServerConnection");

        info!("Nuevo servidor replica con ID {}", msg.id);

        if self.disconnected_servers.contains(&msg.id) {
            debug!("Entro a eliminar un ID repetido");
            self.disconnected_servers.retain(|&x| x != msg.id);
            debug!("{:?}", self.disconnected_servers);
        }

        // Insertar el servidor replica en el diccionario
        self.servers.insert(msg.id, msg.addr.clone());
        self.udp_sockets_replicas.insert(msg.id, msg.socket.clone());

        for addr_replica in self.servers.values() {
            info!("{}", "Enviando pong a TODAS las replicas\n".green());
            if let Err(e) = _ctx.address().try_send(Pong {
                addr: addr_replica.clone(),
            }) {
                error!("Error al enviar el mensaje: {}", e);
            }
        }
    }
}

#[async_handler]
impl Handler<Ping> for Server {
    type Result = Result<(), String>;

    async fn handle(&mut self, _msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
        // Mando ping al lider solo si no soy el lider
        if self.leader_connection.is_none() {
            return Ok(());
        }

        if !self.im_leader {
            debug!("Server::Ping");
            if let Some(leader_connection) = self.leader_connection.clone() {
                let leader_connected = leader_connection.connected();
                if !leader_connected {
                    info!("Lider {} desconectado", self.id_leader);
                    self.leader_connection = None;
                    self.election_on_course = true;
                    if let Err(e) = _ctx.address().try_send(StartElection {}) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                } else {
                    // Si el lider esta conectado, envio el ping
                    let serialized_msg = serialize_message("ping", ());

                    if let Err(e) =
                        leader_connection.try_send(TcpMessage::new(serialized_msg.clone()))
                    {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Handler<Pong> for Server {
    type Result = ();

    /// Envia Pong a los servidores replicas.
    /// El Pong contiene todo el estado de la red.
    fn handle(&mut self, msg: Pong, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::Pong");

        let network_state = UpdateNetworkState {
            ids_count: self.ids_count,
            customers: self.customers.clone(),
            deliveries: self
                .delivery_workers_info
                .iter()
                .map(|d| (d.id.to_string(), d.clone()))
                .collect(),
            restaurants: self.restaurants.clone(),
            udp_sockets_replicas: self.udp_sockets_replicas.clone(),
            disconnected_servers: self.disconnected_servers.clone(),
        };

        let serialized_msg = serialize_message("pong", network_state);
        info!("{}", "Enviando pong a una replica\n".green());
        if let Err(e) = msg.addr.try_send(TcpMessage::new(serialized_msg)) {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}

impl Handler<UpdateNetworkState> for Server {
    type Result = ();

    /// Recibe el estado de la red, y actualiza su estado local.
    fn handle(&mut self, msg: UpdateNetworkState, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::UpdateNetworkState");

        self.udp_sockets_replicas = msg.udp_sockets_replicas;
        self.disconnected_servers = msg.disconnected_servers;
        self.customers = msg.customers;
        self.delivery_workers_info = msg.deliveries.into_iter().map(|(_id, info)| info).collect();
        self.restaurants = msg.restaurants;
        self.clients_connected = msg.ids_count;
    }
}

impl Handler<SetLeader> for Server {
    type Result = ();

    /// Setea la conexion con el servidor lider.
    fn handle(&mut self, msg: SetLeader, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::SetLeader\n");
        self.id_leader = msg.id_leader;
        self.leader_connection = Some(msg.leader_connection);
    }
}

impl Handler<StartElection> for Server {
    type Result = ();

    fn handle(&mut self, _msg: StartElection, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::StartElection");
        info!(
            "Marcando server {} como desconectado por StartElection",
            self.id_leader
        );
        self.disconnected_servers.push(self.id_leader);

        let neighbors_connected = self.update_neighbor(self.actual_neighbor_id);
        if !neighbors_connected {
            if let Err(e) = _ctx.address().try_send(NewLeader {
                id_sender: self.id,
                leader_id: self.id,
                sequence_number: self.sequence_number,
            }) {
                error!("Error al enviar el mensaje: {}", e);
            }
            self.sequence_number += 1;
            return;
        }

        let neighbor_address = get_address(self.actual_neighbor_id);

        println!("ID del lider: {:?}", self.id_leader);
        println!("ID del vecino: {:?}", self.actual_neighbor_id);
        println!("Address del vecino: {:?}", neighbor_address);

        debug!(
            "Soy la replica de ID {} mando election a {}\n",
            self.id, neighbor_address
        );

        let server_ids = vec![self.id];
        let election_message = Election {
            server_ids,
            disconnected_leader_id: self.id_leader,
            sequence_number: self.sequence_number,
        };
        self.sequence_number += 1;
        let serialized_msg = serialize_message("election", election_message.clone());
        self.sequence_number += 1;
        if let Some(udp_sender) = self.udp_sender.as_ref() {
            if let Err(e) = udp_sender.try_send(UdpMessage {
                message: serialized_msg.clone(),
                dst: neighbor_address.clone(),
            }) {
                error!("Error al enviar el mensaje: {}", e);
            }
        }
        if let Err(e) = _ctx.address().try_send(WaitForNeighborAck {
            neighbor_id: self.actual_neighbor_id,
            serialized_msg,
            sequence_number: election_message.sequence_number,
        }) {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}

#[async_handler]
impl Handler<WaitForNeighborAck> for Server {
    type Result = ();
    async fn handle(&mut self, msg: WaitForNeighborAck, _ctx: &mut Self::Context) {
        actix::clock::sleep(Duration::from_millis(300)).await;
        if let Err(e) = _ctx.address().try_send(CheckIfNeighborAcked {
            neighbor_id: msg.neighbor_id,
            serialized_msg: msg.serialized_msg,
            sequence_number: msg.sequence_number,
        }) {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}

#[async_handler]
impl Handler<CheckIfNeighborAcked> for Server {
    type Result = ();
    async fn handle(&mut self, msg: CheckIfNeighborAcked, _ctx: &mut Self::Context) {
        if self.message_received_ack.contains_key(&msg.sequence_number) {
            self.received_neighbor_ack = false;
            return;
        }
        info!("No ACK recibido, marcando vecino desconectado y reenviando");
        self.disconnected_servers.push(msg.neighbor_id);
        let neighbors_connected = self.update_neighbor(self.actual_neighbor_id);
        if !neighbors_connected {
            info!("No hay replicas conectadas, soy el lider");
            // Me mando NewLeader a mi misma
            if let Err(e) = _ctx.address().try_send(NewLeader {
                id_sender: self.id,
                leader_id: self.id,
                sequence_number: self.sequence_number,
            }) {
                error!("Error al enviar el mensaje: {}", e);
            }
            self.sequence_number += 1;
            return;
        }
        debug!(
            "No se recibio ACK de {} de parte de id {}, reenviando a id {}",
            msg.serialized_msg, msg.neighbor_id, self.actual_neighbor_id
        );
        let next_neighbor_address = get_address(self.actual_neighbor_id);
        // Reenvio el mensaje original a el siguiente vecino
        if let Some(udp_sender) = self.udp_sender.as_ref() {
            if let Err(e) = udp_sender.try_send(UdpMessage {
                message: msg.serialized_msg.clone(),
                dst: next_neighbor_address,
            }) {
                error!("Error al enviar el mensaje: {}", e);
            }
        }
        // Espero el ACK del siguiente vecino
        if let Err(e) = _ctx.address().try_send(WaitForNeighborAck {
            serialized_msg: msg.serialized_msg,
            neighbor_id: self.actual_neighbor_id,
            sequence_number: msg.sequence_number,
        }) {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}

impl Handler<Election> for Server {
    type Result = ();

    fn handle(&mut self, msg: Election, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::Election");
        info!(
            "Marcando server {} como desconectado porque me llega de election",
            msg.disconnected_leader_id
        );
        // Marco el server lider como desconectado
        self.disconnected_servers.push(msg.disconnected_leader_id);
        // Si el lider actual es el que se cayo, seteo la conexion como None
        if self.id_leader == msg.disconnected_leader_id {
            self.leader_connection = None;
        }
        // actualizo mi siguiente vecino
        let neighbors_connected = self.update_neighbor(self.actual_neighbor_id);
        // Si no hay replicas conectadas entonces soy el lider
        if !neighbors_connected {
            info!("No hay replicas conectadas, soy el lider");
            if let Err(e) = _ctx.address().try_send(NewLeader {
                id_sender: self.id,
                leader_id: self.id,
                sequence_number: self.sequence_number,
            }) {
                error!("Error al enviar el mensaje: {}", e);
            }
            self.sequence_number += 1;
            return;
        }

        let neighbor_address = get_address(self.actual_neighbor_id);

        println!("ID del lider: {:?}", self.id_leader);
        println!("ID del vecino: {:?}", self.actual_neighbor_id);
        println!("Address del vecino: {:?}", neighbor_address);

        // Si mi id esta en el mensaje election es porque yo lo envie y me llego de vuelta.
        // Es decir, terminó el algoritmo
        let election_finished = msg.server_ids.contains(&self.id);
        if election_finished {
            let mut leader_id = 0;
            // Determino el lider buscando el mayor id en la lista de ids
            for id in msg.server_ids {
                if id > leader_id {
                    leader_id = id;
                }
            }
            info!("Nuevo lider elegido: {leader_id}\n");
            self.id_leader = leader_id;
            let new_leader = NewLeader {
                leader_id,
                id_sender: self.id,
                sequence_number: self.sequence_number,
            };
            self.sequence_number += 1;
            let serialized_msg = serialize_message("new_leader", new_leader.clone());
            // Le mando el mensaje new_leader a mi vecino
            if let Err(e) = self
                .udp_sender
                .as_ref()
                .expect("Error al intentar mandar un mensaje UDP por el UdpSender")
                .try_send(UdpMessage {
                    message: serialized_msg.clone(),
                    dst: neighbor_address,
                })
            {
                error!("Error al enviar el mensaje: {}", e);
            }
            // Espero por el ACK de mi vecino
            if let Err(e) = _ctx.address().try_send(WaitForNeighborAck {
                neighbor_id: self.actual_neighbor_id,
                serialized_msg,
                sequence_number: new_leader.sequence_number,
            }) {
                error!("Error al enviar el mensaje: {}", e);
            }
            return;
        }
        // Si la eleccion no termino, reenvio el mensaje Election a mi vecino
        debug!(
            "Soy la replica de ID {} mando election a {}\n",
            self.id, neighbor_address
        );
        // Agrego mi ID a la lista de IDs
        let mut server_ids = msg.server_ids;
        server_ids.push(self.id);
        let election_msg = Election {
            server_ids,
            disconnected_leader_id: msg.disconnected_leader_id,
            sequence_number: self.sequence_number,
        };
        self.sequence_number += 1;
        let serialized_msg = serialize_message("election", election_msg.clone());

        // Mando Election a mi vecino
        if let Err(e) = self
            .udp_sender
            .as_ref()
            .expect("UdpSender es None en Server")
            .try_send(UdpMessage {
                message: serialized_msg.clone(),
                dst: neighbor_address,
            })
        {
            error!("Error al enviar el mensaje: {}", e);
        }
        // Espero el ACK de mi vecino
        if let Err(e) = _ctx.address().try_send(WaitForNeighborAck {
            neighbor_id: self.actual_neighbor_id,
            serialized_msg,
            sequence_number: election_msg.sequence_number,
        }) {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}

#[async_handler]
impl Handler<NewLeader> for Server {
    type Result = ();
    async fn handle(&mut self, msg: NewLeader, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::NewLeader");

        self.id_leader = msg.leader_id;

        // Si soy el lider y antes no era lider, entonces me pongo a escuchar conexiones
        if msg.leader_id == self.id && !self.im_leader {
            info!("Soy el lider");
            self.im_leader = true;
            // Escucho conexiones
            let connection_listener = ConnectionListener {
                socket: id_to_tcp_address(self.id),
                server_addr: _ctx.address(),
            }
            .start();
            if let Err(e) = connection_listener.try_send(Listen {}) {
                error!("Error al enviar el mensaje: {}", e);
            }
        }
        // Reenvio el mensaje solo si no soy el sender original
        if msg.id_sender != self.id {
            println!("Reenvio new_leader");
            let neighbors_connected = self.update_neighbor(self.actual_neighbor_id);
            // Si no hay replicas conectadas entonces soy el lider
            if !neighbors_connected {
                info!("No hay replicas conectadas, soy el lider");
                if let Err(e) = _ctx.address().try_send(NewLeader {
                    id_sender: self.id,
                    leader_id: self.id,
                    sequence_number: self.sequence_number,
                }) {
                    error!("Error al enviar el mensaje: {}", e);
                }
                self.sequence_number += 1;
            // Si hay replicas conectadas entonces reenvio el mensaje NewLeader a mi vecino
            } else {
                let neighbor_address = get_address(self.actual_neighbor_id);

                let election_message = serialize_message("new_leader", msg.clone());
                if let Err(e) = self
                    .udp_sender
                    .as_ref()
                    .expect("UdpSender no esta seteado en Server")
                    .try_send(UdpMessage {
                        message: election_message.clone(),
                        dst: neighbor_address,
                    })
                {
                    error!("Error al enviar el mensaje: {}", e);
                }
                // Espero el ACK de mi vecino
                if let Err(e) = _ctx.address().try_send(WaitForNeighborAck {
                    neighbor_id: self.actual_neighbor_id,
                    serialized_msg: election_message,
                    sequence_number: msg.sequence_number,
                }) {
                    error!("Error al enviar el mensaje: {}", e);
                }
            }
        }
        // Si no soy lider, me conecto al lider
        if msg.leader_id != self.id && self.leader_connection.is_none() {
            if let Err(e) = _ctx.address().try_send(ConnectToServer {}) {
                error!("Error al enviar el mensaje: {}", e);
            }
        }
    }
}

#[async_handler]
impl Handler<NeighborAck> for Server {
    type Result = ();

    /// Marca un numero de secuencia como ACKed.
    async fn handle(&mut self, msg: NeighborAck, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::NeighborAck");
        // saco al server de disconnected
        self.disconnected_servers.retain(|id| *id != msg.id);
        // marco que recibi el ACK para el sequence number
        self.message_received_ack.insert(msg.sequence_number, true);
        debug!("Received {} ACK from id {}", msg.message, msg.id)
    }
}

/// Crea una nueva conexión TCP. En caso de error, reintenta 5 veces.
/// Luego de reintentar la conexión 5 veces, devuelve error.
async fn create_tcp_connection(socket: String) -> Result<TcpStream, io::Error> {
    info!("create_tcp_connection");
    let mut connection_result = TcpStream::connect(socket.clone()).await;
    let mut count = 0;
    while connection_result.is_err() {
        info!("Fallo la conexion al nuevo lider, reintentando");
        actix::clock::sleep(Duration::from_millis(100)).await;
        if count == 5 {
            return connection_result;
        }
        connection_result = TcpStream::connect(socket.clone()).await;
        count += 1;
    }
    let stream_server_r = connection_result.expect("");
    Ok(stream_server_r)
}

#[async_handler]
impl Handler<ConnectToServer> for Server {
    type Result = ();
    /// Se conecta al servidor lider
    async fn handle(&mut self, _msg: ConnectToServer, _ctx: &mut Self::Context) -> Self::Result {
        actix::clock::sleep(Duration::from_millis(100)).await;
        info!("Conectandome al server lider!");
        let leader_address = id_to_tcp_address(self.id_leader);
        // Intento conectarme al lider varias veces
        let stream_server_r = create_tcp_connection(leader_address).await;
        if stream_server_r.is_err() {
            error!("Fallo al conectarse al lider.");
            return;
        }
        let stream_server_r = stream_server_r.expect("No deberia fallar");
        info!("{}", "Conectado al servidor lider\n".green());

        let (read_server_r, write_half_server_r) = split(stream_server_r);

        // Inicializacion del server
        let server_connection = ServerConnection::create(|ctx| {
            ServerConnection::add_stream(
                LinesStream::new(BufReader::new(read_server_r).lines()),
                ctx,
            );
            let write = Some(write_half_server_r);
            ServerConnection {
                addr: _ctx.address(),
                write,
                response_time: Instant::now(),
            }
        });

        info!("Reconectado al nuevo lider");
        self.leader_connection = Some(server_connection);
        self.election_on_course = false;
    }
}

impl Handler<Stop> for Server {
    type Result = ();
    /// Este mensaje se usa para detener los servers desde el test
    fn handle(&mut self, _msg: Stop, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::Stop");
        _ctx.stop();
    }
}

impl Handler<SendLeaderId> for Server {
    type Result = ();
    /// Este mensaje se usa para obtener el ID del lider desde el test
    fn handle(&mut self, msg: SendLeaderId, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Server::SendLeaderId");
        let message = self.id_leader.to_string();
        if let Err(e) = self
            .udp_sender
            .as_ref()
            .expect("UdpSender no esta seteado en Server")
            .try_send(UdpMessage {
                message,
                dst: msg.addr,
            })
        {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}
