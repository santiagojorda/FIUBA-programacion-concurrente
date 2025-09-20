use crate::delivery::DeliveryWorker;
use crate::messages::{HandshakeReceived, SaveLoginDeliveryInfo};
use actix::{Actor, Addr, Context, StreamHandler};
use app_utils::utils::{deserialize_payload, deserialize_tcp_message};
use log::error;
use std::io::Error;
use std::net::SocketAddr;

pub struct TcpReceiver {
    pub client_addr: Addr<DeliveryWorker>,
    pub addr: SocketAddr,
}

impl Actor for TcpReceiver {
    type Context = Context<Self>;
}

impl StreamHandler<Result<String, Error>> for TcpReceiver {
    fn handle(&mut self, msg: Result<String, Error>, _ctx: &mut Context<Self>) {
        if let Ok(mut line) = msg {
            line = line.trim().to_string();

            if line == "HANDSHAKE" {
                self.client_addr.do_send(HandshakeReceived);
                return;
            };

            let (title, payload) = deserialize_tcp_message(&line);

            match title.as_str() {
                "login_successful_delivery" => {
                    // 1) deserializar cada campo
                    let Ok(id) = deserialize_payload::<u64>(&payload, "id") else {
                        error!("[DELIVERY] id inválido en login_successful_delivery");
                        return;
                    };

                    // 2) reenviar al DeliveryWorker
                    self.client_addr.do_send(SaveLoginDeliveryInfo { id });
                }
                _ => {
                    error!(
                        "[DELIVERY] Mensaje '{}' no reconocido desde {:?}",
                        title, self.addr
                    );
                }
            }
        } else if let Err(e) = msg {
            error!("[DELIVERY] Error en stream desde {:?}: {}", self.addr, e);
        }
    }
}
