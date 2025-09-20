use crate::messages::{
    BusyDeliveryWorker, CustomerConnected, DeliveryStatus, DeliveryWorkerConnected,
    FreeDeliveryWorker, GetDeliveries, GetRestaurants, NewRestaurant, NewServerConnection, Pong,
    RegisterDelivery, RequestDelivery,
};
use crate::server::Server;
use actix::prelude::*;
use actix_async_handler::async_handler;
use app_utils::utils::{deserialize_payload, deserialize_tcp_message};
use log::{error, info};
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;

#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpMessage(pub String);

#[derive(Message)]
#[rtype(result = "()")]
pub struct _CloseConnection;

impl TcpMessage {
    pub fn new<S: Into<String>>(string: S) -> Self {
        let mut string = string.into();
        if !string.ends_with('\n') {
            string.push('\n');
        }
        TcpMessage(string)
    }
}

/// Estructura del envío TCP.
pub struct TcpSender {
    pub write: Option<WriteHalf<TcpStream>>,
    pub addr: SocketAddr,
    pub server_addr: Addr<Server>,
    pub handshake_sent: bool,
}

impl Actor for TcpSender {
    type Context = Context<Self>;
}

#[async_handler]
impl Handler<TcpMessage> for TcpSender {
    type Result = ();

    async fn handle(&mut self, msg: TcpMessage, _ctx: &mut Context<Self>) {
        let mut writer = self.write.take().expect("Error al escribir");
        let ret = async move {
            writer
                .write_all(msg.0.as_bytes())
                .await
                .expect("Error al escribir");
            writer
        }
        .await;
        self.write = Some(ret);
    }
}

impl StreamHandler<Result<String, std::io::Error>> for TcpSender {
    fn handle(&mut self, msg: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(line) => {
                if !self.handshake_sent {
                    self.handshake_sent = true;
                    ctx.address().do_send(TcpMessage::new("HANDSHAKE"));

                    info!("[SERVER]: Handshake enviado a {:?}", self.addr);
                } else {
                    //ACA PROCESA OTROS MENSAJES DE CLIENTE
                    //aca deberia deserializar el mensaje recibido. Este tiene un header con el texto que representa el mensaje y luego el payload, que son los datos de cada struct que viene en el json
                    let (title, payload) = deserialize_tcp_message(&line);
                    match title.as_str() {
                        "login" => {
                            let Ok(name) = deserialize_payload::<String>(&payload, "name") else {
                                error!("Error al deserializar el campo name");
                                return;
                            };

                            let complete_customer_login = CustomerConnected {
                                name,
                                addr: ctx.address(),
                            };

                            if let Err(e) = self.server_addr.try_send(complete_customer_login) {
                                error!("Error al enviar el mensaje: {}", e);
                            }
                        }
                        "ACK" => {
                            info!(
                                "[SERVER]: ACK recibido de {:?}. Cliente conectado",
                                self.addr
                            );
                        }
                        "new_restaurant" => {
                            let Ok(socket) = deserialize_payload::<String>(&payload, "socket")
                            else {
                                error!("Error al deserializar el campo socket");
                                return;
                            };
                            let Ok(name) = deserialize_payload::<String>(&payload, "name") else {
                                error!("Error al deserializar el campo name");
                                return;
                            };
                            let Ok(position) =
                                deserialize_payload::<(u64, u64)>(&payload, "position")
                            else {
                                error!("Error al deserializar el campo position");
                                return;
                            };
                            let new_rest = NewRestaurant {
                                socket,
                                name,
                                position,
                            };

                            if let Err(e) = self.server_addr.try_send(new_rest) {
                                error!("Error al enviar el mensaje: {}", e);
                            }
                        }
                        "login_delivery" => {
                            let Ok(name) = deserialize_payload::<String>(&payload, "name") else {
                                error!("Error al deserializar el campo name");
                                return;
                            };
                            let complete_delivery_login = DeliveryWorkerConnected {
                                name: name,
                                addr: ctx.address(),
                            };
                            if let Err(e) = self.server_addr.try_send(complete_delivery_login) {
                                error!("Error al enviar el mensaje: {}", e);
                            }
                        }
                        "register_delivery" => {
                            // parsea posición y socket
                            let position = deserialize_payload::<(u64, u64)>(&payload, "position")
                                .expect("position inválido en register_delivery");
                            let socket = deserialize_payload::<String>(&payload, "socket")
                                .expect("socket inválido en register_delivery");

                            // despacha al actor Server
                            let msg = RegisterDelivery {
                                position,
                                socket,
                                sender_addr: ctx.address(),
                            };
                            self.server_addr
                                .try_send(msg)
                                .expect("no pudo enviar RegisterDelivery al Server");
                        }
                        "get_deliveries" => {
                            let id = deserialize_payload::<u64>(&payload, "id")
                                .expect("get_deliveries: id inválido");
                            let position = deserialize_payload::<(u64, u64)>(&payload, "position")
                                .expect("get_deliveries: position inválido");

                            let msg = GetDeliveries {
                                id,
                                position,
                                addr: ctx.address(),
                            };
                            if let Err(e) = self.server_addr.try_send(msg) {
                                error!("[SERVER] fallo al enviar GetDeliveries: {}", e);
                            }
                        }
                        "get_restaurants" => {
                            let Ok(id) = deserialize_payload::<u64>(&payload, "id") else {
                                error!("Error al deserializar el campo id");
                                return;
                            };
                            let Ok(position) =
                                deserialize_payload::<(u64, u64)>(&payload, "position")
                            else {
                                error!("Error al deserializar el campo position");
                                return;
                            };

                            let get_restaurants = GetRestaurants {
                                _customer_id: id,
                                position: position,
                                addr: ctx.address(),
                            };

                            if let Err(e) = self.server_addr.try_send(get_restaurants) {
                                error!("Error al enviar el mensaje: {}", e);
                            }
                        }
                        "delivery_status" => {
                            let customer_id = deserialize_payload::<u64>(&payload, "customer_id")
                                .expect("delivery_status sin customer_id");
                            let message = deserialize_payload::<String>(&payload, "message")
                                .expect("delivery_status sin message");
                            let ds = DeliveryStatus {
                                customer_id,
                                message,
                            };
                            let _ = self.server_addr.do_send(ds);
                        }
                        "request_delivery" => {
                            let delivery_id = deserialize_payload::<u64>(&payload, "delivery_id")
                                .expect("request_delivery sin delivery_id");
                            let order_id = deserialize_payload::<u64>(&payload, "order_id")
                                .expect("request_delivery sin order_id");
                            let restaurant_position =
                                deserialize_payload::<(u64, u64)>(&payload, "restaurant_position")
                                    .expect("request_delivery sin restaurant_position");
                            let customer_position =
                                deserialize_payload::<(u64, u64)>(&payload, "customer_position")
                                    .expect("request_delivery sin customer_position");

                            // reenvío al actor Server para que lo tramite
                            let rd = RequestDelivery {
                                delivery_id,
                                order_id,
                                restaurant_position,
                                customer_position,
                            };
                            let _ = self.server_addr.do_send(rd);
                        }
                        "busy_delivery" => {
                            // 1) extraemos el id
                            let busy_id: u64 = deserialize_payload(&payload, "id")
                                .expect("busy_delivery sin campo id");
                            // 2) le enviamos al actor Server
                            if let Err(e) = self
                                .server_addr
                                .try_send(BusyDeliveryWorker { id: busy_id })
                            {
                                error!("[SERVER] fallo al enviar BusyDeliveryWorker: {}", e);
                            }
                        }
                        "free_delivery" => {
                            let id = deserialize_payload::<u64>(&payload, "id").unwrap();
                            let position =
                                deserialize_payload::<(u64, u64)>(&payload, "position").unwrap();
                            let socket = deserialize_payload::<String>(&payload, "socket").unwrap();
                            if let Err(e) = self.server_addr.try_send(FreeDeliveryWorker {
                                id,
                                position,
                                socket,
                            }) {
                                error!("[SERVER] Error enviando FreeDeliveryWorker: {}", e);
                            }
                        }
                        "new_replica" => {
                            let Ok(id) = deserialize_payload::<u64>(&payload, "id") else {
                                error!("Error al deserializar el campo id");
                                return;
                            };
                            let Ok(socket) = deserialize_payload::<String>(&payload, "socket")
                            else {
                                error!("Error al deserializar el campo socket");
                                return;
                            };
                            let msg = NewServerConnection {
                                id: id,
                                socket,
                                addr: ctx.address(),
                            };
                            if let Err(e) = self.server_addr.try_send(msg) {
                                error!("[SERVER] Error al enviar NewServerConnection: {}", e);
                            }
                        }
                        "ping" => {
                            // El ping es un mensaje de control, no requiere procesamiento adicional
                            info!("[SERVER]: Ping recibido de una replica {:?}", self.addr);
                            if let Err(e) = self.server_addr.try_send(Pong {
                                addr: ctx.address(),
                            }) {
                                error!("[SERVER] Error al enviar pong: {}", e);
                            }
                        }
                        _ => {
                            error!("[SERVER]: Mensaje {title} invalido");
                        }
                    }
                }
            }
            Err(err) => {
                error!("[{:?}] Error de lectura: {:?}", self.addr, err);
            }
        }
    }
}
