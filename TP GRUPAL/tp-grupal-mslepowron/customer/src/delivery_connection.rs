use crate::customer::Customer;
use crate::messages::StatusUpdate;
use crate::sender::TcpMessage;
use actix::{Actor, Addr, Context, Handler, StreamHandler};
use actix_async_handler::async_handler;
use app_utils::utils::{deserialize_payload, deserialize_tcp_message};
use std::io::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;

pub struct DeliveryConnection {
    pub write: Option<WriteHalf<TcpStream>>,
    pub client_addr: Addr<Customer>,
    pub socket_addr: SocketAddr,
}

impl Actor for DeliveryConnection {
    type Context = Context<Self>;
}

impl StreamHandler<Result<String, Error>> for DeliveryConnection {
    fn handle(&mut self, msg: Result<String, Error>, _ctx: &mut Context<Self>) {
        if let Ok(raw) = msg {
            let line = raw.trim();
            let (title, payload) = deserialize_tcp_message(line);
            if title == "status_update" {
                match deserialize_payload::<String>(&payload, "message") {
                    Ok(txt) => {
                        self.client_addr.do_send(StatusUpdate { message: txt });
                    }
                    Err(e) => {
                        log::error!("[DELIVERY-CONN] payload.status_update sin message: {}", e)
                    }
                }
            } else {
                log::error!("[DELIVERY-CONN] Mensaje P2P no reconocido: {}", title);
            }
        } else if let Err(e) = msg {
            log::error!("[DELIVERY-CONN] Error en stream P2P: {}", e);
        }
    }
}

///el Sender de un customer se usa para comunicarse con el servidor.
/// su tarea principal es convertir mensajes de tipo String en TcpMessage, serialziarlos y enviarlos al servidor.
#[async_handler]
impl Handler<TcpMessage> for DeliveryConnection {
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
