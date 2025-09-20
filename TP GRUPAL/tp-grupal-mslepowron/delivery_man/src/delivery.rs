use crate::messages::{
    HandshakeReceived, LoginDelivery, NewConnection, RegisterDelivery, RequestDelivery,
    RestaurantConnectionMessage, SaveLoginDeliveryInfo, SetReceiver, StartDelivery,
};
use crate::receiver::TcpReceiver;
use crate::restaurant_connection::RestaurantConnection;
use crate::sender::{TcpMessage, TcpSender};
use actix::clock::sleep;
use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, ResponseActFuture, WrapFuture,
};
use actix_async_handler::async_handler;
use app_utils::utils::serialize_message;
use colored::Colorize;
use log::{debug, error, info};
use serde_json::json;
use server::messages::{BusyDeliveryWorker, FreeDeliveryWorker};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Estructura que representa un repartidor, con su ID, posición y disponibilidad para tomar un pedido
pub struct DeliveryWorker {
    pub name: String,
    pub id: u64,
    pub worker_position: (u64, u64),
    pub restaurant_position: Option<(u64, u64)>,
    pub customer_position: Option<(u64, u64)>,
    pub available: bool,
    pub tcp_sender: Addr<TcpSender>,
    pub tcp_server_receiver: Option<Addr<TcpReceiver>>,
    pub handshake_done: bool,
    pub connections: Vec<Addr<RestaurantConnection>>,
    pub delivery_socket: String,
    pub customer_socket: Option<String>,
}

impl Actor for DeliveryWorker {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let msg = TcpMessage::new(format!("DeliveryWorker {} conectado", self.id));

        self.tcp_sender.do_send(msg);
    }
}

/// Mensaje que recibe del servidor, indicando que este reconoció que el delivery quiere entablar una conexion Tcp, y aceptó la misma.
impl Handler<HandshakeReceived> for DeliveryWorker {
    type Result = ();

    fn handle(&mut self, msg: HandshakeReceived, _ctx: &mut Context<Self>) {
        self.handshake_done = true;

        // Enviar ACK real al servidor a través del sender
        let serialized_ack_msg = serialize_message("ACK", msg);
        self.tcp_sender
            .do_send(TcpMessage::new(serialized_ack_msg.clone()));
    }
}

impl Handler<SetReceiver> for DeliveryWorker {
    type Result = ();

    fn handle(&mut self, msg: SetReceiver, _ctx: &mut Context<Self>) {
        self.tcp_server_receiver = Some(msg.receiver);
    }
}

///Indica que el delivery se debe loggear al server
#[async_handler]
impl Handler<LoginDelivery> for DeliveryWorker {
    type Result = ();

    fn handle(&mut self, msg: LoginDelivery, _ctx: &mut Self::Context) -> Self::Result {
        let serialized_msg = serialize_message("login_delivery", msg);

        if let Err(e) = self
            .tcp_sender
            .try_send(TcpMessage::new(serialized_msg.clone()))
        {
            error!("Error al enviar el mensaje: {}", e);
        }

        sleep(Duration::from_secs(1)).await;
    }
}

///Confirmacion de registro del delivery en el server
impl Handler<RegisterDelivery> for DeliveryWorker {
    type Result = ();

    fn handle(&mut self, msg: RegisterDelivery, _ctx: &mut Context<Self>) {
        let payload = json!({
            "id":        self.id,
            "position": msg.position,
            "socket":   msg.delivery_socket,
        });
        let text = serialize_message("register_delivery", payload);
        self.tcp_sender.do_send(TcpMessage::new(text));

        info!(
            "{}",
            format!(
                "[DELIVERY]: Registrando en servidor → position={:?}, socket={}",
                msg.position, msg.delivery_socket
            )
            .magenta()
        );
    }
}

///Guarda la informacion que el server le settea para reconocerlo
#[async_handler]
impl Handler<SaveLoginDeliveryInfo> for DeliveryWorker {
    type Result = ();

    fn handle(&mut self, msg: SaveLoginDeliveryInfo, _ctx: &mut Self::Context) -> Self::Result {
        info!("{}", "[DELIVERY]: Login complete\n".magenta());

        self.id = msg.id;
    }
}

///Recibe una conexion de un restaurante, quien le solicitara la entrega de un pedido
impl Handler<NewConnection> for DeliveryWorker {
    type Result = ();

    fn handle(&mut self, msg: NewConnection, _ctx: &mut Self::Context) -> Self::Result {
        self.connections.push(msg.addr);
    }
}

///Un restaurante le solicita entregar un pedido. La respuesta se determian segun el delivery este ocupado o no.
impl Handler<RequestDelivery> for DeliveryWorker {
    type Result = ();

    fn handle(&mut self, msg: RequestDelivery, _ctx: &mut Self::Context) -> Self::Result {
        debug!("DeliveryWorker::RequestDelivery\n");

        // Si el repartidor no está disponible, no puede tomar el pedido
        if !self.available {
            let serialized_msg = serialize_message("no", ());
            let response = RestaurantConnectionMessage::new(serialized_msg);
            if let Err(e) = msg.addr.try_send(response) {
                error!("Error al enviar el mensaje: {}", e);
            }
            info!(
                "{}",
                format!(
                    "[DELIVERY]: NO disponible para tomar el pedido #{}",
                    msg.id_order
                )
                .blue()
            )
        } else {
            // El repartidor esta disponible, responde "ok" y loggea el camino del pedido
            self.id = msg.id_order;
            self.restaurant_position = Some(msg.restaurant_position);
            self.customer_position = Some(msg.customer_position);
            self.customer_socket = Some(msg.customer_socket);
            self.available = false; // Marcar como no disponible para nuevos pedidos
            let serialized_msg = serialize_message(
                "ok",
                json!({"delivery_socket": self.delivery_socket, "order_id": msg.id_order, "delivery_name": self.name}),
            );
            let response = RestaurantConnectionMessage::new(serialized_msg);

            info!(
                "{}",
                format!(
                    "[DELIVERY]: Se acepto el pedido #{} del restaurante en: {:?}",
                    msg.id_order, msg.restaurant_position
                )
                .blue()
            );

            if let Err(e) = msg.addr.try_send(response) {
                error!("Error al enviar el mensaje: {}", e);
            }

            // Le aviso al server que estoy ocupado para que no reparta mi socket
            let busy_delivery_msg = BusyDeliveryWorker { id: self.id };

            let serialized_msg = serialize_message("busy_delivery", busy_delivery_msg);

            if let Err(e) = self
                .tcp_sender
                .try_send(TcpMessage::new(serialized_msg.clone()))
            {
                error!("Error al enviar el mensaje: {}", e);
            }

            if let Err(e) = _ctx.address().try_send(StartDelivery {}) {
                error!("Error al enviar el mensaje: {}", e);
            }
        }
    }
}

///Indica que debe informarle al customer que está en camino a su direccion para entregar el pedido
#[async_handler]
impl Handler<StartDelivery> for DeliveryWorker {
    type Result = ();

    async fn handle(&mut self, _msg: StartDelivery, _ctx: &mut Context<Self>) {
        let customer_socket = self
            .customer_socket
            .clone()
            .expect("falta customer_socket en StartDelivery");
        let delivery_id = self.id.clone();
        let delivery_position = self.worker_position.clone();
        let delivery_socket = self.delivery_socket.clone();
        let tcp_sender = self.tcp_sender.clone();

        info!(
            "{}",
            format!(
                "[DELIVERY]: Yendo a repartir el pedido #{} a la direccion: {:?}",
                delivery_id, delivery_position
            )
            .magenta()
        );

        let delivery_id_clone = delivery_id.clone();
        let delivery_position_clone = delivery_position.clone();
        let delivery_socket_clone = delivery_socket.clone();
        let tcp_sender_clone = tcp_sender.clone();

        _ctx.spawn(
            async move {
                match TcpStream::connect(&customer_socket).await {
                    Ok(mut stream) => {
                        for text in &[
                            "Llegando al restaurante",
                            "Pedido recogido",
                            "Pedido entregado",
                        ] {
                            let framed =
                                serialize_message("status_update", json!({ "message": text }));
                            let framed = format!("{}\n", framed);
                            if let Err(e) = stream.write_all(framed.as_bytes()).await {
                                log::error!("[DELIVERY]: Error enviando mensaje al cliente: {}", e);
                                break;
                            }
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                    Err(e) => {
                        debug!("[DELIVERY]: No se pudo conectar con el cliente: {}", e);
                        //return;
                    }
                }
            }
            .into_actor(self),
        );
        self.available = true;
        // Marcar al repartidor como libre y notificar al servidor
        let free_msg = serialize_message(
            "free_delivery",
            FreeDeliveryWorker {
                id: delivery_id_clone,
                position: delivery_position_clone,
                socket: delivery_socket_clone.clone(),
            },
        );
        tcp_sender_clone.do_send(TcpMessage::new(free_msg));
    }
}
