use crate::delivery_connection::DeliveryConnection;
use crate::gateway_connection::GatewayConnection;
use crate::messages::{
    Abort, AddDeliveryConnection, AuthPaymentResponse, CommitOrder, CommitPayment,
    DeliveryAccepted, GetRestaurants, HandshakeReceived, Login, NearbyRestaurants, OrderCanceled,
    OrderConfirmed, OrderReadyToPickup, PaymentConfirmed, PaymentDenied, PrepareOrder,
    PreparePayment, PreparingOrder, RecoverCustomerOrder, RecoverData, RecoverDelivery,
    RestaurantOrderResponse, SaveLoginInfo, SetReceiver, StatusUpdate,
};
use crate::receiver::TcpReceiver;
use crate::sender::{TcpMessage, TcpSender};
use actix::clock::sleep;
use actix::{Actor, Addr, AsyncContext, Context, Handler, StreamHandler, WrapFuture};
use actix_async_handler::async_handler;
use app_utils::payment_type::PaymentType;
use app_utils::utils::serialize_message;
use colored::Colorize;
use log::{debug, error, info};
use rand::Rng;
use serde_json::{Value, json};
use server::apps_info::restaurant_info::RestaurantData;
use std::io;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, split};
use tokio::net::TcpStream;
use tokio_stream::wrappers::LinesStream;

/// Estructura que representa un cliente, con su ID, nombre, posición, restaurante elegido y tipo de pago
pub struct Customer {
    pub id: u64,
    ///nombre del cliente
    pub name: String,
    ///posicion del cliente; a donde se debe entregar el pedido
    pub position: (u64, u64),
    ///ubicacion del restaurante elegido
    pub chosen_restaurant: Option<(u64, u64)>,
    ///id de la order, asignada por el restaurante
    pub id_order: u64,
    ///pago en efectivo/tarjeta
    pub payment_type: PaymentType,
    ///estado de autorizacion de pago por el gateway
    pub payment_authorized: bool,
    ///estado de confirmacion de pago por el gateway
    pub payment_confirmed: bool,
    ///sender para comunicartse con el servidor
    pub tcp_sender: Addr<TcpSender>,
    ///receiver para recibir informacion de otros actores a traves de mensajes Tcp
    pub tcp_server_receiver: Option<Addr<TcpReceiver>>,
    ///handshake - conexion con el servidor
    pub handshake_done: bool,
    ///datos de los restaurantes disponibles en la applicacion
    pub available_restaurants: Vec<RestaurantData>,
    ///socket para comunciarse con el restaurante al que se realizara el pedido
    pub selected_restaurant_socket: Option<String>,
    ///informacion para comunicarse con los restaurantes disponibles
    pub restaurant_connections: Vec<Addr<TcpSender>>,
    ///sender para enviarle mensajes Tcp al actor Restaurant
    pub restaurant_sender: Option<Addr<TcpSender>>,
    pub restaurant_receiver: Option<Addr<TcpReceiver>>,
    ///Conexion al gateway
    pub gateway: Addr<GatewayConnection>,
    ///two phase-commit authorizations
    pub waiting_gateway_auth: bool,
    pub waiting_restaurant_auth: bool,
    ///two phase-commit gateway response
    pub gateway_vote: Option<bool>,
    ///two phase-commit restaurant response
    pub restaurant_vote: Option<bool>,
    pub delivery_socket: String,
    ///indica si se es un cliente nuevo (false) o se esta recuperando de una caida y quiere reestablecer sus datos (true)
    pub recover_customer_data: bool,
    pub assigned_delivery_socket: Option<String>,
    pub delivery_connections: Vec<Addr<DeliveryConnection>>,
}

impl Customer {
    ///Logica de seleccion de un restaurante que se encuentre en su zona
    fn select_restaurant(&mut self) -> Option<RestaurantData> {
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..self.available_restaurants.len());
        let chosen_restaurant = self.available_restaurants[idx].clone();
        Some(chosen_restaurant)
    }

    ///Guardado del estado del customer
    async fn save_customer_info(
        id: usize,
        name: String,
        available_restaurants: Vec<RestaurantData>,
        selected_restaurant_socket: Option<String>,
        payment_type: PaymentType,
        payment_authorized: bool,
        payment_confirmed: bool,
        id_order: u64,
        assigned_delivery_socket: Option<String>,
    ) {
        let customer_file_name = format!("client_{}.json", name);

        if let Ok(mut customer_file) = tokio::fs::File::create(customer_file_name).await {
            let write_result = customer_file
                .write_all(
                    json!({
                        "id": id,
                        "name": name,
                        "available_restaurants": available_restaurants,
                        "selected_restaurant_socket": selected_restaurant_socket,
                        "payment_type": payment_type.to_string(),
                        "payment_authorized": payment_authorized,
                        "payment_confirmed": payment_confirmed,
                        "id_order": id_order,
                        "assigned_delivery_socket": assigned_delivery_socket,

                    })
                    .to_string()
                    .as_bytes(),
                )
                .await;
            if let Err(err) = write_result {
                debug!("Fallo al guardar estado en el archivo. Error: {err}");
            }
        }
    }

    ///Restauracion de la informacion de un customer que se esta recuperando
    pub fn restore_from_json(&mut self, data: &Value) {
        if let Some(id) = data["id"].as_u64() {
            self.id = id;
        }

        if let Some(name) = data["name"].as_str() {
            self.name = name.to_string();
        }

        if let Some(order_id) = data["id_order"].as_u64() {
            self.id_order = order_id;
        }

        if let Some(ptype) = data["payment_type"].as_str() {
            self.payment_type = match ptype {
                "Cash" => PaymentType::Cash,
                "CreditCard" => PaymentType::CreditCard,
                _ => self.payment_type,
            };
        }

        if let Some(authorized) = data["payment_authorized"].as_bool() {
            self.payment_authorized = authorized;
        }

        if let Some(confirmed) = data["payment_confirmed"].as_bool() {
            self.payment_confirmed = confirmed;
        }

        if let Some(socket) = data["selected_restaurant_socket"].as_str() {
            self.selected_restaurant_socket = Some(socket.to_string());
        }

        if let Some(restaurants) = data["available_restaurants"].as_array() {
            if let Ok(rest_list) =
                serde_json::from_value::<Vec<RestaurantData>>(Value::Array(restaurants.clone()))
            {
                self.available_restaurants = rest_list;
            }
        }

        if let Some(socket) = data["assigned_delivery_socket"].as_str() {
            self.assigned_delivery_socket = Some(socket.to_string());
        }

        self.recover_customer_data = false;
    }

    ///Conexion al restaurante seleccionado
    pub async fn restaurant_connection(
        restaurant_socket: String,
        client_addr: Addr<Customer>,
    ) -> Result<Addr<TcpSender>, io::Error> {
        async {
            let stream = TcpStream::connect(restaurant_socket).await?;

            info!("{}", "Conectado al restaurante\n".blue());

            let local_addr = stream
                .local_addr()
                .expect("Error al obtener socket local luego de reconexion con el server");
            let (read, write_half) = split(stream);
            TcpReceiver::create(|ctx| {
                TcpReceiver::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
                TcpReceiver {
                    client_addr,
                    addr: local_addr,
                }
            });
            Ok(TcpSender {
                write: Some(write_half),
                addr: local_addr,
            }
            .start())
        }
        .await
    }

    ///Conexion al delivery que le entregara el pedido
    pub async fn delivery_connection(
        delivery_socket: String,
        client_addr: Addr<Customer>,
    ) -> Result<Addr<DeliveryConnection>, io::Error> {
        async {
            let stream = TcpStream::connect(delivery_socket).await?;

            let local_addr = stream
                .local_addr()
                .expect("Error al obtener socket local luego de reconexion con el server");
            let (read, write_half) = split(stream);
            info!("[CUSTOMER]: Cliente escuchando repartidores en el socket:");
            let addr = DeliveryConnection::create(|ctx| {
                // agregamos la línea al actor para que reciba mensajes
                DeliveryConnection::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
                DeliveryConnection {
                    write: Some(write_half),
                    client_addr: client_addr.clone(),
                    socket_addr: local_addr,
                }
            });
            Ok(addr)
        }
        .await
    }

    ///Logica de aprobacion del pago y aceptacion del pedido en un commit de dos fases.
    fn check_2pc_decision(&mut self, _ctx: &mut <Self as Actor>::Context) {
        if let (Some(gateway_ok), Some(restaurant_ok)) = (self.gateway_vote, self.restaurant_vote) {
            if gateway_ok && restaurant_ok {
                let total_price = rand::thread_rng().gen_range(10.0..100.0);

                info!("{}", format!("[CUSTOMER]: Pago Autorizado 💳").yellow());

                self.payment_authorized = true;

                let serialized_payment_msg = serialize_message(
                    "commit_payment",
                    CommitPayment {
                        customer_id: self.id,
                        amount: total_price,
                    },
                );
                let _ = self
                    .gateway
                    .try_send(TcpMessage::new(serialized_payment_msg.clone()));

                info!(
                    "{}",
                    format!("[CUSTOMER]: El Restaurante ha aceptado tu pedido! 🍽️").blue()
                );
                println!(
                    "[DEBUG-CLIENT] Enviando commit_order con socket = {}",
                    self.delivery_socket.clone()
                );
                if let Some(sender) = &self.restaurant_sender {
                    let serialized_order = serialize_message(
                        "commit_order",
                        CommitOrder {
                            customer_id: self.id,
                            payment_type: self.payment_type,
                            customer_position: self.position,
                            amount: total_price,
                            customer_socket: self.delivery_socket.clone(),
                        },
                    );
                    let _ = sender.try_send(TcpMessage::new(serialized_order.clone()));
                }
            } else {
                info!("Al menos uno rechazó. Enviando ABORT.");
                let abort_message = serialize_message("abort", Abort {});
                if let Some(sender) = &self.restaurant_sender {
                    let _ = sender.try_send(TcpMessage::new(abort_message.clone()));
                }
                let _ = self
                    .gateway
                    .try_send(TcpMessage::new(abort_message.clone()));

                // Reintentar solo una vez
                // Reset votes
                self.gateway_vote = None;
                self.restaurant_vote = None;

                actix::spawn({
                    let customer_addr = _ctx.address();
                    let nearby = self.available_restaurants.clone();
                    async move {
                        sleep(Duration::from_secs(1)).await;
                        let _ = customer_addr.try_send(NearbyRestaurants {
                            nearby_restaurants: nearby,
                        });
                    }
                });
            }
        }
    }
}

impl Actor for Customer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let msg = TcpMessage::new(format!("Customer {} conectado", self.name));

        self.tcp_sender.do_send(msg);
    }
}

impl Handler<HandshakeReceived> for Customer {
    type Result = ();

    fn handle(&mut self, msg: HandshakeReceived, _ctx: &mut Context<Self>) {
        self.handshake_done = true;

        // Enviar ACK real al servidor a través del sender
        let serialized_ack_msg = serialize_message("ACK", msg);
        self.tcp_sender
            .do_send(TcpMessage::new(serialized_ack_msg.clone()));
    }
}

/// Se utiliza para almacenar la direccion del actor TcpReceiver que se utiliza para comunicarse con el servidor
impl Handler<SetReceiver> for Customer {
    type Result = ();

    fn handle(&mut self, msg: SetReceiver, _ctx: &mut Self::Context) -> Self::Result {
        self.tcp_server_receiver = Some(msg.receiver);
    }
}

#[async_handler]
impl Handler<Login> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: Login, _ctx: &mut Self::Context) -> Self::Result {
        let serialized_msg = serialize_message("login", msg);

        if let Err(e) = self
            .tcp_sender
            .try_send(TcpMessage::new(serialized_msg.clone()))
        {
            error!("Error al enviar el mensaje: {}", e);
        }

        actix::clock::sleep(Duration::from_millis(2000)).await;
    }
}

impl Handler<SaveLoginInfo> for Customer {
    type Result = ();

    fn handle(&mut self, msg: SaveLoginInfo, _ctx: &mut Self::Context) -> Self::Result {
        let client_name = self.name.clone();
        info!(
            "{}",
            format!("[CUSTOMER]: {} Login complete", client_name)
                .to_string()
                .green()
        );

        self.id = msg.id;
    }
}

#[async_handler]
impl Handler<AddDeliveryConnection> for Customer {
    type Result = ();

    async fn handle(
        &mut self,
        msg: AddDeliveryConnection,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        self.delivery_connections.push(msg.addr);
    }
}

///Solicita al server los restaurantes disponible segun la ubicacion del comensal.
///El Server unicamente le disponibilizara los restaurantes que esten en un rango cercano al comensal.

#[async_handler]
impl Handler<GetRestaurants> for Customer {
    type Result = ();

    async fn handle(&mut self, _msg: GetRestaurants, _ctx: &mut Self::Context) -> Self::Result {
        info!("{}", "Buscando restaurantes... 🍔\n".green());

        let id = self.id;
        let position = self.position;

        let serialized_msg =
            serialize_message("get_restaurants", json!({"id": id, "position": position}));

        if let Err(e) = self
            .tcp_sender
            .try_send(TcpMessage::new(serialized_msg.clone()))
        {
            error!("Error al enviar el mensaje: {}", e);
        }

        //CHEQUEOS POR ESTADO DEL SERVIDOR
        sleep(Duration::from_secs(1)).await;
    }
}

#[async_handler]
impl Handler<NearbyRestaurants> for Customer {
    type Result = ();

    async fn handle(
        &mut self,
        msg: NearbyRestaurants,
        _ctx: &mut <Customer as Actor>::Context,
    ) -> Self::Result {
        //SAVE CUSTOMER INFO

        self.available_restaurants = msg.nearby_restaurants.clone();

        Customer::save_customer_info(
            self.id as usize,
            self.name.clone(),
            self.available_restaurants.clone(),
            self.selected_restaurant_socket.clone(),
            self.payment_type,
            self.payment_authorized,
            self.payment_confirmed,
            self.id_order,
            self.assigned_delivery_socket.clone(),
        )
        .await;

        //selecciona aleatoriamente un restaurante de la lista de restaurantes disponibles
        if let Some(selected_restaurant) = self.select_restaurant() {
            let restaurant_addr_str = selected_restaurant.socket.clone();
            let customer_addr = _ctx.address();

            let gateway_addr = self.gateway.clone();
            self.selected_restaurant_socket = Some(restaurant_addr_str.clone());

            let rest_connection =
                Customer::restaurant_connection(restaurant_addr_str, customer_addr).await;

            match rest_connection {
                Ok(chosen_restaurant) => {
                    self.restaurant_sender = Some(chosen_restaurant.clone());
                    //2PC Prepare Payment (Payment Auth)
                    info!("{}", format!("[CUSTOMER]: Realizando un pedido").green());
                    let gateway_addr_clone = gateway_addr.clone();
                    let chosen_restaurant_clone = chosen_restaurant.clone();

                    _ctx.spawn(actix::fut::wrap_future(async move {
                        actix::clock::sleep(Duration::from_millis(2000)).await;

                        let prepare_payment_msg =
                            serialize_message("prepare_payment", PreparePayment {});
                        let prepare_order_msg = serialize_message("prepare_order", PrepareOrder {});

                        // Enviar PREPARE a ambos
                        if let Err(e) = gateway_addr_clone
                            .try_send(TcpMessage::new(prepare_payment_msg.clone()))
                        {
                            error!("Error enviando PreparePayment: {}", e);
                        }
                        if let Err(e) = chosen_restaurant_clone
                            .try_send(TcpMessage::new(prepare_order_msg.clone()))
                        {
                            error!("Error enviando PrepareOrder: {}", e);
                        }
                    }));
                }
                Err(_e) => {
                    error!(
                        "{}",
                        "[CUSTOMER]: Error! No fue posible conectarse al restaurante".red()
                    );
                }
            }

            // if let Some(delivery_socket) = self.assigned_delivery_socket.clone() {
            //     let _ = self
            //         .delivery_connection(delivery_socket, _ctx.address())
            //         .await;
            // }
        } else {
            println!("No hay restaurantes cercanos disponibles.");
        }
    }
}

///Respuesta al mensaje "Prepare" durante el commit de dos faases que envia el coordinador Customer. Indica si el gateway autoriza o no el pago
#[async_handler]
impl Handler<AuthPaymentResponse> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: AuthPaymentResponse, _ctx: &mut Self::Context) {
        self.gateway_vote = Some(msg.accepted);
        self.check_2pc_decision(_ctx);
    }
}

#[async_handler]
///Respuesta al mensaje "Prepare" durante el commit de dos faases que envia el coordinador Customer
impl Handler<RestaurantOrderResponse> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: RestaurantOrderResponse, ctx: &mut Self::Context) {
        self.restaurant_vote = Some(msg.accepted);
        self.check_2pc_decision(ctx);
    }
}

///Indica que el cobro fue confirmado por el gateway.
#[async_handler]
impl Handler<PaymentConfirmed> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: PaymentConfirmed, _ctx: &mut Self::Context) {
        info!("{}", format!("Se ha confirmado el pago.✅"));

        Customer::save_customer_info(
            self.id as usize,
            self.name.clone(),
            self.available_restaurants.clone(),
            self.selected_restaurant_socket.clone(),
            self.payment_type,
            self.payment_authorized,
            self.payment_confirmed,
            self.id_order,
            self.assigned_delivery_socket.clone(),
        )
        .await;

        match self.payment_type {
            PaymentType::CreditCard => {
                info!(
                    "{}",
                    format!("[CUSTOMER]: Le pagaste: ${:.2} al restaurante.", msg.amount).yellow()
                );
                self.payment_confirmed = true;
            }
            PaymentType::Cash => {
                info!(
                    "{}",
                    format!(
                        "[CUSTOMER]: Deberas pagarle: ${:.2} al repartidor cuando te entregue tu pedido",
                        msg.amount
                    )
                    .yellow()
                );
                self.payment_confirmed = false;
            }
        }
    }
}

///En caso de denegacion de pago se pide la lista de restaurantes nuevamente.
#[async_handler]
impl Handler<PaymentDenied> for Customer {
    type Result = ();

    async fn handle(&mut self, _msg: PaymentDenied, _ctx: &mut Self::Context) {
        info!("{}", format!("Se ha denegado el pago.🚫"));
        self.payment_confirmed = false;

        //save state
        Customer::save_customer_info(
            self.id as usize,
            self.name.clone(),
            self.available_restaurants.clone(),
            self.selected_restaurant_socket.clone(),
            self.payment_type,
            self.payment_authorized,
            self.payment_confirmed,
            self.id_order,
            self.assigned_delivery_socket.clone(),
        )
        .await;
    }
}

///El restaurante acepto el pedido. Espera notificaciones de
/// el estado de su orden enviadas por el restaurante.
#[async_handler]
impl Handler<OrderConfirmed> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: OrderConfirmed, ctx: &mut Self::Context) {
        info!(
            "{}",
            format!(
                "[CUSTOMER]: El restaurante ha confirmado tu orden 🛎️ Nro de orden: #{}",
                msg.order_id
            )
            .blue()
        );

        self.id_order = msg.order_id;

        Customer::save_customer_info(
            self.id as usize,
            self.name.clone(),
            self.available_restaurants.clone(),
            self.selected_restaurant_socket.clone(),
            self.payment_type,
            self.payment_authorized,
            self.payment_confirmed,
            self.id_order,
            self.assigned_delivery_socket.clone(),
        )
        .await;

        actix::clock::sleep(Duration::from_millis(2000)).await;

        info!(
            "{}",
            format!("[CUSTOMER]: Esperando notificaciones del restaurante sobre mi pedido")
        );
    }
}

///Notificacion sobre que el Restaurante cancela el pedido.
/// El Customer vuelve a buscar un restaurante para realizar una nueva orden.
#[async_handler]
impl Handler<OrderCanceled> for Customer {
    type Result = ();

    async fn handle(&mut self, _msg: OrderCanceled, _ctx: &mut Self::Context) {
        actix::clock::sleep(Duration::from_millis(2000)).await;
        info!(
            "{}",
            format!("[CUSTOMER]: Lo sentimos! El restaurante ha cancelado tu pedido 😓").blue()
        );
        sleep(Duration::from_secs(1)).await;
    }
}

///Actualizacion de status de la orden: El restaurante esta preparando el pedido
#[async_handler]
impl Handler<PreparingOrder> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: PreparingOrder, _ctx: &mut Self::Context) {
        actix::clock::sleep(Duration::from_millis(2000)).await;
        info!(
            "{}",
            format!(
                "[CUSTOMER]: El restaurante esta preparando tu pedidio: #{} 👩🏻‍🍳🍜",
                msg.order_id
            )
            .blue()
        );
    }
}

///Actualizacion de status de la orden: El restaurante eesperando que un delivery la busque
#[async_handler]
impl Handler<OrderReadyToPickup> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: OrderReadyToPickup, _ctx: &mut Self::Context) {
        actix::clock::sleep(Duration::from_millis(2000)).await;
        info!(
            "{}",
            format!(
                "[CUSTOMER]: Tu orden: #{} ya esta lista! El restaurante esta esperando a un repartidor 🔜",
                msg.order_id
            )
            .blue()
        );
    }
}

///Un delivery acepto recoger el pedido y se lo va a entregar al cliente
#[async_handler]
impl Handler<DeliveryAccepted> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: DeliveryAccepted, ctx: &mut Context<Self>) {
        let customer_addr_for_notify = ctx.address();
        let customer_addr_for_delivery = customer_addr_for_notify.clone();

        // Este es el socket del delivery
        let delivery_socket = msg.delivery_socket.clone();

        self.assigned_delivery_socket = Some(delivery_socket.clone());

        info!(
            "{}",
            format!(
                "[CUSTOMER]: El delivery: {} va a venir a retirar el pedido al restaurante!",
                msg.delivery_name
            )
            .magenta()
        );

        let id = self.id as usize;
        let name = self.name.clone();
        let available_restaurants = self.available_restaurants.clone();
        let selected_restaurant_socket = self.selected_restaurant_socket.clone();
        let payment_type = self.payment_type;
        let payment_authorized = self.payment_authorized;
        let payment_confirmed = self.payment_confirmed;
        let id_order = self.id_order;
        let assigned_delivery_socket = self.assigned_delivery_socket.clone();

        ctx.spawn(
            async move {
                match TcpStream::connect(&delivery_socket).await {
                    Ok(stream) => {
                        let (read, write) = split(stream);
                        DeliveryConnection::create(move |delivery_ctx| {
                            delivery_ctx.add_stream(LinesStream::new(BufReader::new(read).lines()));
                            DeliveryConnection {
                                write: Some(write),
                                client_addr: customer_addr_for_delivery,
                                socket_addr: delivery_socket
                                    .parse()
                                    .expect("Invalid delivery socket address"),
                            }
                        });

                        // Guardo la info del cliente actualizada
                        Customer::save_customer_info(
                            id,
                            name,
                            available_restaurants,
                            selected_restaurant_socket,
                            payment_type,
                            payment_authorized,
                            payment_confirmed,
                            id_order,
                            assigned_delivery_socket,
                        )
                        .await;
                    }
                    Err(e) => {
                        log::error!("No se pudo conectar al socket del delivery: {}", e);
                    }
                }
            }
            .into_actor(self),
        );
    }
}

///Recibe actualizaciones del estado de la entrega por parte del repartidor
#[async_handler]
impl Handler<StatusUpdate> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: StatusUpdate, _ctx: &mut Self::Context) {
        info!("{}", format!("[DELIVERY STATUS] {}", msg.message).magenta());
    }
}

///Indica que debe reestablecer sus datos con los almacenados en el archivo de persistencia. Se reconecta con los actores necesarios para continuar el flujo del pedido
#[async_handler]
impl Handler<RecoverData> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: RecoverData, _ctx: &mut Self::Context) {
        self.restore_from_json(&msg.customer_status);

        if !self.payment_authorized {
            info!("El pago no esta autorizado. Se reintentara realziar un pedido");
            if let Err(e) = _ctx.address().try_send(GetRestaurants {}) {
                error!("Error al enviar el mensaje: {}", e);
            }
        } else if self.payment_confirmed {
            info!("El pago estaba confirmado. La orden se encontraba aprobada");

            //Si el pago estaba confirmado, el restaurante
            //tambien habia aceptado la orden (por el commit de dos fases). Le pido al restaurante el status de mi orden
            if let Some(rest_socket) = self.selected_restaurant_socket.clone() {
                let order_id = self.id_order.clone();

                let restaurant =
                    Customer::restaurant_connection(rest_socket, _ctx.address().clone()).await;

                if let Ok(restaurant) = restaurant {
                    let recover_customer_order = serialize_message(
                        "recover_customer_order",
                        RecoverCustomerOrder { order_id },
                    );

                    if let Err(e) =
                        restaurant.try_send(TcpMessage::new(recover_customer_order.clone()))
                    {
                        error!("Error enviando PrepareOrder: {}", e);
                    }
                } else {
                    println!("NO se pudo coenctar al socket del rest");
                }
            }
        }
    }
}

///Recupera la conexion con un delivery frente a una caida
#[async_handler]
impl Handler<RecoverDelivery> for Customer {
    type Result = ();

    async fn handle(&mut self, msg: RecoverDelivery, _ctx: &mut Self::Context) {
        // if let Some(delivery_socket) = self.assigned_delivery_socket.clone() {
        //     let order_id = self.id_order.clone();

        //     let delivery =
        //         Customer::delivery_connection(delivery_socket, _ctx.address().clone()).await;

        //     if let Ok(delivery) = delivery {
        //         let recover_customer_delivery = serialize_message(
        //             "recover_customer_delivery",
        //             RecoverCustomerOrder { order_id },
        //         );

        //         if let Err(e) =
        //             delivery.try_send(TcpMessage::new(recover_customer_delivery.clone()))
        //         {
        //             error!("Error enviando PrepareOrder: {}", e);
        //         }
        //     } else {
        //         println!("NO se pudo coenctar al socket del delivery");
        //     }
        // }

        info!(
            "{}",
            format!("[DELIVERY STATUS]: Pedido recogido").magenta()
        );
        actix::clock::sleep(Duration::from_secs(5)).await;
        info!(
            "{}",
            format!("[DELIVERY STATUS]: Pedido entregado").magenta()
        );
    }
}
