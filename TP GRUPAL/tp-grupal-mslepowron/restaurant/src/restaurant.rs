use crate::customer_connection::CustomerConnection;
use crate::delivery_connection::DeliveryConnection;
use crate::messages::{
    AcceptDelivery, GetDeliveries, HandshakeReceived, NearbyDeliveries, NewCustomerConnection,
    NewOrder, OrderConfirmed, OrderReadyToPickup, PrepareOrder, RecoverDelivery, RecoverOrderData,
    RegisterRestaurant, SetReceiver, TryNextDelivery,
};
use crate::receiver::TcpReceiver;
use crate::sender::{TcpMessage, TcpSender};
use actix::{Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, StreamHandler, fut};
use actix_async_handler::async_handler;
use app_utils::utils::serialize_message;
use colored::Colorize;
use log::{debug, error, info};
use order::Order;
use order::order::{OrderStatus, SerializableOrder};
use rand::Rng;
use serde_json::json;
use server::apps_info::delivery_info::DeliveryInfo;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, split};
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_stream::wrappers::LinesStream;

/// Estructura que representa un restaurante, con su ID, posición y lista de pedidos
pub struct Restaurant {
    pub name: String,
    pub restaurant_id: u64,
    pub position: (u64, u64),
    pub orders_queue: Vec<Order<Addr<CustomerConnection>>>,
    pub next_order_id: u64, // ID para el próximo pedido
    pub customers_connected: Vec<Addr<CustomerConnection>>,
    pub tcp_sender: Addr<TcpSender>, //comunicacion con el servidor
    pub tcp_server_receiver: Option<Addr<TcpReceiver>>, //comunicacion con el servidor
    pub handshake_done: bool,
    pub restaurant_socket: String,
    pub pending_deliveries: Vec<DeliveryInfo>,
    pub has_deliveries: bool, // Indica si hay repartidores disponibles
    pub current_order_id: Option<u64>, // ID del pedido actual en preparación
    pub current_customer_socket: Option<String>, // Socket del cliente actual
    pub current_customer_position: Option<(u64, u64)>, // Posición del cliente actual
}

impl Restaurant {
    ///Logica que determina si el restaurante acepta una orden o no
    pub fn accept_new_order() -> bool {
        let mut rng = rand::thread_rng();
        rng.gen_bool(0.9) // 90% de aceptar, 10% de rechazar (false)
    }

    ///Se guarda el estado del restaurante
    async fn save_restaurant_info(
        name: String,
        restaurant_id: u64,
        position: (u64, u64),
        orders_queue: Vec<SerializableOrder>,
        next_order_id: u64, // ID para el próximo pedido
        restaurant_socket: String,
    ) {
        let restaurant_file_name = format!("rest_{}.json", name);

        if let Ok(mut restaurant_file) = tokio::fs::File::create(restaurant_file_name).await {
            let write_result = restaurant_file
                .write_all(
                    json!({
                        "restaurant_id": restaurant_id,
                        "name": name,
                        "position": position,
                        "orders_queue": orders_queue,
                        "next_order_id": next_order_id,
                        "restaurant_socket": restaurant_socket,
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
}

impl Actor for Restaurant {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let msg = TcpMessage::new(format!("Restaurant {} conectado", self.restaurant_id));

        self.tcp_sender.do_send(msg);
    }
}

///Ack del handshake con el server
impl Handler<HandshakeReceived> for Restaurant {
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
impl Handler<SetReceiver> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: SetReceiver, _ctx: &mut Self::Context) -> Self::Result {
        self.tcp_server_receiver = Some(msg.receiver);
    }
}

///El restaurante se comunica con el servidor para registrarse en el sistema de pedidos.
#[async_handler]
impl Handler<RegisterRestaurant> for Restaurant {
    type Result = ();

    async fn handle(&mut self, msg: RegisterRestaurant, _ctx: &mut Self::Context) -> Self::Result {
        let serialized_msg = serialize_message("new_restaurant", msg);

        if let Err(e) = self
            .tcp_sender
            .try_send(TcpMessage::new(serialized_msg.clone()))
        {
            error!("Error al enviar el mensaje: {}", e);
        }

        //CHEQUEAR ESTO SENDER SERVER

        let serializable_orders: Vec<SerializableOrder> = self
            .orders_queue
            .iter()
            .map(|order| SerializableOrder {
                order_id: order.order_id,
                total_price: order.total_price,
                order_status: order.order_status.clone(),
                customer_position: order.customer_position,
            })
            .collect();

        Restaurant::save_restaurant_info(
            self.name.clone(),
            self.restaurant_id,
            self.position,
            serializable_orders,
            self.next_order_id,
            self.restaurant_socket.clone(),
        )
        .await;
    }
}

///Mensaje de un Customer que se quiere conectar con el restaurante para realizar un pedido.
impl Handler<NewCustomerConnection> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: NewCustomerConnection, _ctx: &mut Self::Context) -> Self::Result {
        info!("[RESTAURANT]: Nuevo cliente conectado:");

        self.customers_connected.push(msg.addr);
    }
}

///El Restaurante solicita al servidor informacion acerca de los deliveries disponibles para retirar un pedido.
#[async_handler]
impl Handler<GetDeliveries> for Restaurant {
    type Result = ();

    async fn handle(&mut self, _msg: GetDeliveries, _ctx: &mut Context<Self>) -> Self::Result {
        info!("{}", "Buscando repartidores disponibles... 🚴‍♂️\n".green());

        let id = self.restaurant_id;
        let position = self.position;

        let serialized =
            serialize_message("get_deliveries", json!({ "id": id, "position": position }));

        if let Err(e) = self.tcp_sender.try_send(TcpMessage::new(serialized)) {
            error!("Error al pedir repartidores: {}", e);
        }

        sleep(Duration::from_secs(1)).await;
    }
}
///El servidor le adjunta los deliveries que estan cerca de la zona del restaurante
impl Handler<NearbyDeliveries> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: NearbyDeliveries, ctx: &mut Context<Self>) {
        // println!(
        //     "[DEBUG] NearbyDeliveries → recibí {} candidates, has_seen_deliveries={}",
        //     msg.nearby_deliveries.len(),
        //     self.has_deliveries
        // );
        // 1) si la lista está vacía...
        if msg.nearby_deliveries.is_empty() {
            if !self.has_deliveries {
                info!(
                    "{}",
                    format!(
                        "[RESTAURANT]: No hay repartidores conectados. Cancelando pedido #{}",
                        self.current_order_id.unwrap_or(0)
                    )
                    .red()
                );
                // nunca hubo repartidores → cancelamos
                if let Some(order) = self.orders_queue.pop() {
                    let cancel_msg =
                        serialize_message("order_canceled", json!({ "order_id": order.order_id }));
                    order.client_connection.do_send(TcpMessage::new(cancel_msg));
                }
            } else {
                // hubo repartidores antes pero todos están ocupados ahora → reintentar
                info!(
                    "{}",
                    format!(
                        "[RESTAURANT]: Todos los repartidore estan ocupados, reintentando en 2s...⏳"
                    )
                    .magenta()
                );
                ctx.run_later(std::time::Duration::from_secs(2), |_act, ctx| {
                    ctx.address().do_send(GetDeliveries);
                });
            }
            return;
        }

        // 2) si llegó con al menos uno, marcamos el flag
        self.has_deliveries = true;

        // 3) cargamos la cola de candidatos y disparamos el primero
        self.pending_deliveries = msg.nearby_deliveries;
        let last = self.orders_queue.last().unwrap();
        self.current_order_id = Some(last.order_id);
        self.current_customer_socket = Some(last.customer_socket.clone());

        info!(
            "[RESTAURANT] Pending deliveries cargados: {}",
            self.pending_deliveries.len()
        );

        ctx.address().do_send(TryNextDelivery);
    }
}

///Recibe un mensaje de un Customer con los detalles de su orden. Registra un nuevo pedido a preparar (confirmado)
/// Le notifica periodicamente al Customer el estado de su orden y actualiza la misma.
#[async_handler]
impl Handler<NewOrder> for Restaurant {
    type Result = ();

    async fn handle(&mut self, msg: NewOrder, _ctx: &mut Context<Self>) {
        let id = self.next_order_id;
        self.next_order_id += 1;
        let total: f64 = rand::thread_rng().gen_range(10.0..100.0);

        // guardo el pedido en la cola con la conexión
        let order = Order {
            order_id: id,
            total_price: total,
            order_status: OrderStatus::OrderConfirmed, // estado inicial
            client_connection: msg.client_conn.clone(),
            customer_position: msg.customer_position,
            customer_socket: msg.customer_socket.clone(),
        };
        self.orders_queue.push(order);

        // println!(
        //     "[DEBUG][NewOrder] Encolada order_id={} total={:.2}. Ahora orders_queue.len()={}",
        //     id,
        //     total,
        //     self.orders_queue.len()
        // );

        let serialized_order_msg =
            serialize_message("order_confirmed", OrderConfirmed { order_id: id });
        if let Err(e) = msg
            .client_conn
            .try_send(TcpMessage::new(serialized_order_msg))
        {
            error!("Error al enviar el mensaje: {}", e);
        }

        let serializable_orders: Vec<SerializableOrder> = self
            .orders_queue
            .iter()
            .map(|order| SerializableOrder {
                order_id: order.order_id,
                total_price: order.total_price,
                order_status: order.order_status.clone(),
                customer_position: order.customer_position,
            })
            .collect();

        Restaurant::save_restaurant_info(
            self.name.clone(),
            self.restaurant_id,
            self.position,
            serializable_orders,
            self.next_order_id,
            self.restaurant_socket.clone(),
        )
        .await;

        if let Err(e) = _ctx.address().try_send(PrepareOrder { order_id: id }) {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}

///Indica que le debe avisar al customer que su orden esta en preparacion
#[async_handler]
impl Handler<PrepareOrder> for Restaurant {
    type Result = ();

    async fn handle(&mut self, msg: PrepareOrder, _ctx: &mut Context<Self>) {
        actix::clock::sleep(Duration::from_millis(800)).await;

        if let Some(order) = self
            .orders_queue
            .iter_mut()
            .find(|o| o.order_id == msg.order_id)
        {
            // Actualizar el estado
            order.order_status = OrderStatus::PreparingOrder;

            let payload = json!({
                "order_id": order.order_id,
            });

            let msg_str = serialize_message("preparing_order", payload);

            if let Err(e) = order.client_connection.try_send(TcpMessage::new(msg_str)) {
                error!(
                    "[RESTAURANT]: Error al enviar 'preparing_order' al cliente: {}",
                    e
                );
            } else {
                info!(
                    "[RESTAURANT]: Notificación de preparación enviada para order_id={}",
                    order.order_id
                );
            }
        } else {
            error!(
                "[RESTAURANT]: No se encontró la orden con ID {} para preparar",
                msg.order_id
            );
        }

        let serializable_orders: Vec<SerializableOrder> = self
            .orders_queue
            .iter()
            .map(|order| SerializableOrder {
                order_id: order.order_id,
                total_price: order.total_price,
                order_status: order.order_status.clone(),
                customer_position: order.customer_position,
            })
            .collect();

        Restaurant::save_restaurant_info(
            self.name.clone(),
            self.restaurant_id,
            self.position,
            serializable_orders,
            self.next_order_id,
            self.restaurant_socket.clone(),
        )
        .await;

        if let Err(e) = _ctx.address().try_send(OrderReadyToPickup {
            order_id: msg.order_id,
        }) {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}

///Indica que debe avisarle al customer que su orden esta lista para ser retirada por un delivery
#[async_handler]
impl Handler<OrderReadyToPickup> for Restaurant {
    type Result = ();

    async fn handle(&mut self, msg: OrderReadyToPickup, _ctx: &mut Context<Self>) {
        actix::clock::sleep(Duration::from_millis(800)).await;

        if let Some(order) = self
            .orders_queue
            .iter_mut()
            .find(|o| o.order_id == msg.order_id)
        {
            // Actualizar el estado
            order.order_status = OrderStatus::PreparingOrder;

            let payload = json!({
                "order_id": order.order_id,
            });

            let msg_str = serialize_message("order_ready_for_pickup", payload);

            if let Err(e) = order.client_connection.try_send(TcpMessage::new(msg_str)) {
                error!(
                    "[RESTAURANT]: Error al enviar 'order_ready_for_pickup' al cliente: {}",
                    e
                );
            } else {
                info!(
                    "[RESTAURANT]: Notificación de pedido listo enviada para order_id={}",
                    order.order_id
                );
            }
        } else {
            error!(
                "[RESTAURANT]: No se encontró la orden con ID {} para entregar",
                msg.order_id
            );
        }

        let serializable_orders: Vec<SerializableOrder> = self
            .orders_queue
            .iter()
            .map(|order| SerializableOrder {
                order_id: order.order_id,
                total_price: order.total_price,
                order_status: order.order_status.clone(),
                customer_position: order.customer_position,
            })
            .collect();

        Restaurant::save_restaurant_info(
            self.name.clone(),
            self.restaurant_id,
            self.position,
            serializable_orders,
            self.next_order_id,
            self.restaurant_socket.clone(),
        )
        .await;

        _ctx.address().do_send(GetDeliveries);
    }
}

///Mensaje informativo al cliente con los datos para que el delivery que entregará su pedido se comunique con el
impl Handler<AcceptDelivery> for Restaurant {
    type Result = ();

    fn handle(&mut self, msg: AcceptDelivery, ctx: &mut Context<Self>) {
        if let Some(order) = self
            .orders_queue
            .iter_mut()
            .find(|o| o.order_id == msg.order_id)
        {
            {
                //let order = self.orders_queue.remove(idx);
                order.order_status = OrderStatus::OutForDelivery;

                let payload = json!({
                    "order_id": msg.order_id,
                    "total_price": order.total_price,
                    "delivery_position": msg.delivery_position,
                    "delivery_socket": msg.delivery_socket,
                    "delivery_name": msg.delivery_name,
                });
                let text = serialize_message("delivery_accepted", payload);

                if let Err(e) = order.client_connection.try_send(TcpMessage::new(text)) {
                    error!(
                        "[RESTAURANT]: Error al enviar 'delivery_accepter_order' al cliente: {}",
                        e
                    );
                } else {
                    info!(
                        "[RESTAURANT]: notificacion de delivery asignado para la order #{}",
                        order.order_id
                    );
                }

                self.pending_deliveries.clear();
                self.current_order_id = None;
                self.current_customer_socket = None;

                // if !self.orders_queue.is_empty() {
                //     ctx.address().do_send(GetDeliveries);
                // }
            }
        }
    }
}

///Reintenta asignar un delivery para el pedido
impl Handler<TryNextDelivery> for Restaurant {
    type Result = ();

    fn handle(&mut self, _msg: TryNextDelivery, ctx: &mut Context<Self>) {
        // 1) si ya no quedan candidatos, reintentamos todo en 3s
        if self.pending_deliveries.is_empty() {
            if let Some(order) = self.orders_queue.last() {
                let order_id = order.order_id;
                log::info!(
                    "[RESTAURANT] Se vació la lista de candidatos para order_id={} → reintentando en 3s",
                    order_id
                );
                let server_addr = ctx.address();
                ctx.run_later(Duration::from_secs(3), move |_actor: &mut Restaurant, _| {
                    server_addr.do_send(GetDeliveries);
                });
            }
            return;
        }

        // 2) tomamos el siguiente candidato P2P
        let info = self.pending_deliveries.remove(0);
        let order_id = self.current_order_id.unwrap();
        let customer_socket = self.current_customer_socket.clone().unwrap();

        let payload = json!({
            "order_id":            order_id,
            "restaurant_position": self.position,
            "customer_position":   self.position,
            "customer_socket":     customer_socket,
        });
        // Asegúrate de terminar el mensaje con '\n'
        let framed = format!("{}\n", serialize_message("request_delivery", payload));

        // capturamos estos antes de ir al async
        let me = ctx.address();
        let socket = info.socket.clone();
        let owner = me.clone(); // Owner para la DeliveryConnection
        let addr: SocketAddr = socket.parse().unwrap();

        ctx.spawn(
            fut::wrap_future(async move {
                // Intentamos la conexión
                TcpStream::connect(&socket).await.ok()
            })
            .map(
                move |maybe_stream: Option<TcpStream>,
                      _actor: &mut Restaurant,
                      _: &mut Context<Restaurant>| {
                    if let Some(stream) = maybe_stream {
                        // 3) si conectó, creamos la conexión directa
                        let (read, write) = split(stream);
                        let conn = DeliveryConnection::create(|conn_ctx| {
                            DeliveryConnection::add_stream(
                                LinesStream::new(BufReader::new(read).lines()),
                                conn_ctx,
                            );
                            DeliveryConnection {
                                write: Some(write),
                                owner: owner.clone(),
                                addr,
                            }
                        });
                        conn.do_send(TcpMessage::new(framed.clone()));
                    } else {
                        // 4) si falla, probamos con el siguiente candidato
                        me.do_send(TryNextDelivery);
                    }
                },
            ),
        );
    }
}

#[async_handler]
impl Handler<RecoverOrderData> for Restaurant {
    type Result = ();

    async fn handle(&mut self, msg: RecoverOrderData, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(order) = self
            .orders_queue
            .iter_mut()
            .find(|o| o.order_id == msg.order_id)
        {
            order.client_connection = msg.addr.clone();

            match order.order_status {
                OrderStatus::OrderConfirmed => {
                    let serialized_order_msg = serialize_message(
                        "order_confirmed",
                        OrderConfirmed {
                            order_id: msg.order_id,
                        },
                    );
                    if let Err(e) = msg.addr.try_send(TcpMessage::new(serialized_order_msg)) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                    if let Err(e) = _ctx.address().try_send(PrepareOrder {
                        order_id: msg.order_id,
                    }) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
                OrderStatus::PreparingOrder => {
                    if let Err(e) = _ctx.address().try_send(PrepareOrder {
                        order_id: msg.order_id,
                    }) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
                OrderStatus::ReadyToPickup => {
                    if let Err(e) = _ctx.address().try_send(OrderReadyToPickup {
                        order_id: msg.order_id,
                    }) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
                _ => {
                    println!("El delivery tiene la order");
                    order.order_status = OrderStatus::OrderDelivered;
                    let serialized_order_msg = serialize_message(
                        "recovery_delivery",
                        RecoverDelivery {
                            order_id: msg.order_id,
                        },
                    );
                    if let Err(e) = msg.addr.try_send(TcpMessage::new(serialized_order_msg)) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                    //En los demas estados del Order, la order la tiene el Delivery.
                    //Habria que mandarle un Mensaje al Delivery diciendo que el cliente
                    //con order id "x" se esta recuperando.
                }
            }
        } else {
            error!(
                "No se encontró la orden {} en el restaurante.",
                msg.order_id
            );
        }
    }
}
