use crate::messages::HandshakeReceived;
use crate::messages::NearbyDeliveries;
use crate::restaurant::Restaurant;
use actix::{Actor, Addr, Context, StreamHandler};
use app_utils::utils::{deserialize_payload, deserialize_tcp_message};
use server::apps_info::delivery_info::DeliveryInfo;
use std::{io::Error, net::SocketAddr};

pub struct TcpReceiver {
    /// Dirección de socket asociada
    pub addr: SocketAddr,
    /// Direccion del actor del restaurante
    pub client_addr: Addr<Restaurant>,
}

impl Actor for TcpReceiver {
    type Context = Context<Self>;
}

impl StreamHandler<Result<String, Error>> for TcpReceiver {
    fn handle(&mut self, msg: Result<String, Error>, _ctx: &mut Context<Self>) {
        if let Ok(line) = msg {
            let line = line.trim();

            if line == "HANDSHAKE" {
                // Enviar ACK al servidor
                self.client_addr.do_send(HandshakeReceived);
            };

            let (title, payload) = deserialize_tcp_message(&line);
            match title.as_str() {
                "nearby_deliveries" => {
                    // deserializa la lista de DeliveryInfo
                    let deliveries: Vec<DeliveryInfo> =
                        deserialize_payload(&payload, "nearby_deliveries")
                            .expect("nearby_deliveries inválido");
                    // envía el mensaje al actor Restaurant
                    self.client_addr.do_send(NearbyDeliveries {
                        nearby_deliveries: deliveries,
                    });
                }
                _ => {
                    log::error!("[RESTAURANT]: Mensaje '{}' inválido", title);
                }
            }
        }
    }
}
