use crate::messages::{Election, NeighborAck, NewLeader, SendLeaderId};
use crate::server::Server;
use actix::{Actor, Addr, AsyncContext, Context, Handler, Message};
use actix_async_handler::async_handler;
use app_utils::utils::{deserialize_payload, serialize_message, split_message};
use core::str;
use log::{debug, error, info};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::net::UdpSocket;

const MAX_PACKET_SIZE: usize = 65535;

/// Representa un actor que recibe mensajes UDP a traves de un socket
pub struct UdpReceiver {
    /// Socket UDP
    pub socket: Arc<UdpSocket>,
    /// Address del actor socket
    pub server: Option<Addr<Server>>,
}

impl Actor for UdpReceiver {
    type Context = Context<Self>;
}

/// Manda a escuchar mensajes UDP al UdpReceiver
#[derive(Message)]
#[rtype(result = "()")]
pub struct Listen {}

/// Mensaje que representa el pedido el envio de un
/// mensaje UDP.
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendMessage {
    pub message: String,
    pub addr: String,
}

/// Mensaje usado para setear la conexion con el server
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetServer {
    pub server_addr: Addr<Server>,
}

impl Handler<SetServer> for UdpReceiver {
    type Result = ();

    fn handle(&mut self, msg: SetServer, _ctx: &mut Context<Self>) {
        self.server = Some(msg.server_addr);
    }
}

#[async_handler]
impl Handler<Listen> for UdpReceiver {
    type Result = ();

    async fn handle(&mut self, _msg: Listen, _ctx: &mut Context<Self>) {
        info!("UdpReceiver::Listen");
        let mut buf = [0; MAX_PACKET_SIZE];
        let socket = self.socket.clone();
        let result = async move {
            let Ok((size, addr)) = socket.recv_from(&mut buf).await else {
                error!("Error al recibir mensaje desde socket UDP, ignorando mensaje");
                return None;
            };
            let Ok(msg_str) = str::from_utf8(&buf[..size]) else {
                error!("Error al convertir los bytes leidos a string, ignorando mensaje");
                return None;
            };
            Some((socket, msg_str.to_string(), addr))
        }
        .await;

        if result.is_none() {
            error!("Error al recibir mensaje UDP, ignorando mensaje");
            let _ = _ctx.address().try_send(Listen {});
            return;
        }

        let (socket, message, addr) = result.expect("Error al recibir mensaje UDP");
        self.socket = socket;
        debug!("Received UDP '{}'", message.trim());

        // Primero, si viene en JSON puro { "name": "...", "payload": {...} }
        if message.trim_start().starts_with('{') {
            if let Ok(v) = serde_json::from_str::<Value>(&message) {
                if let Some(name) = v
                    .get("title")
                    .or_else(|| v.get("name"))
                    .and_then(Value::as_str)
                {
                    match name {
                        "election" => {
                            // deserializar como antes...
                            let payload = &v["payload"];
                            let server_ids = serde_json::from_value(payload["server_ids"].clone())
                                .unwrap_or_default();
                            let disconnected_leader_id =
                                serde_json::from_value(payload["disconnected_leader_id"].clone())
                                    .unwrap_or(0);
                            let sequence_number =
                                serde_json::from_value(payload["sequence_number"].clone())
                                    .unwrap_or(0);
                            let election_msg = Election {
                                server_ids,
                                disconnected_leader_id,
                                sequence_number,
                            };
                            // mandar ACK por UDP
                            let serialized_ack = serialize_message(
                                "ack",
                                json!({"sequence_number": sequence_number, "message": message, "id": 0}),
                            );
                            let _ = _ctx.address().try_send(SendMessage {
                                addr: addr.to_string(),
                                message: serialized_ack,
                            });
                            // reenviar al server interno
                            let _ = self.server.as_ref().unwrap().try_send(election_msg);
                        }
                        "new_leader" => {
                            let payload = &v["payload"];
                            let leader_id = payload["leader_id"].as_u64().unwrap_or(0);
                            let id_sender = payload["id_sender"].as_u64().unwrap_or(0);
                            let sequence_number = payload["sequence_number"].as_u64().unwrap_or(0);
                            let new_leader = NewLeader {
                                leader_id,
                                id_sender,
                                sequence_number,
                            };
                            let serialized_ack = serialize_message(
                                "ack",
                                json!({"sequence_number": sequence_number, "message": message, "id": 0}),
                            );
                            let _ = _ctx.address().try_send(SendMessage {
                                addr: addr.to_string(),
                                message: serialized_ack,
                            });
                            let _ = self.server.as_ref().unwrap().try_send(new_leader);
                        }
                        "ack" => {
                            let payload = &v["payload"];
                            let id = payload["id"].as_u64().unwrap_or(0);
                            let msg_str =
                                payload["message"].as_str().unwrap_or_default().to_string();
                            let sequence_number = payload["sequence_number"].as_u64().unwrap_or(0);
                            let _ = self.server.as_ref().unwrap().try_send(NeighborAck {
                                id,
                                message: msg_str,
                                sequence_number,
                            });
                        }
                        "get_leader" => {
                            info!("[UDP] get_leader recibido");
                            let _ = self.server.as_ref().unwrap().try_send(SendLeaderId {
                                addr: addr.to_string(),
                            });
                        }
                        other => {
                            debug!("UDP mensaje JSON desconocido: {}", other);
                        }
                    }
                }
                // volver a escuchar
                let _ = _ctx.address().try_send(Listen {});
                return;
            }
        }

        // Si no es JSON puro, seguimos con el split_message normal:
        let (msg_name, payload) = split_message(&message);
        match msg_name.as_str() {
            "election" => {
                debug!("UdpReceiver::election");
                let server_ids: Vec<u64> =
                    deserialize_payload(&payload, "server_ids").unwrap_or_default();
                let disconnected_leader_id: u64 =
                    deserialize_payload(&payload, "disconnected_leader_id").unwrap_or(0);
                let sequence_number: u64 =
                    deserialize_payload(&payload, "sequence_number").unwrap_or(0);
                let election_msg = Election {
                    server_ids,
                    disconnected_leader_id,
                    sequence_number,
                };
                let serialized_ack = serialize_message(
                    "ack",
                    json!({"sequence_number": sequence_number, "message": message, "id": 0}),
                );
                let _ = _ctx.address().try_send(SendMessage {
                    addr: addr.to_string(),
                    message: serialized_ack,
                });
                let _ = self.server.as_ref().unwrap().try_send(election_msg);
            }
            "new_leader" => {
                debug!("UdpReceiver::new_leader");
                let leader_id: u64 = deserialize_payload(&payload, "leader_id").unwrap_or(0);
                let id_sender: u64 = deserialize_payload(&payload, "id_sender").unwrap_or(0);
                let sequence_number: u64 =
                    deserialize_payload(&payload, "sequence_number").unwrap_or(0);
                let new_leader = NewLeader {
                    leader_id,
                    id_sender,
                    sequence_number,
                };
                let serialized_ack = serialize_message(
                    "ack",
                    json!({"sequence_number": sequence_number, "message": message, "id": 0}),
                );
                let _ = _ctx.address().try_send(SendMessage {
                    addr: addr.to_string(),
                    message: serialized_ack,
                });
                let _ = self.server.as_ref().unwrap().try_send(new_leader);
            }
            "ack" => {
                debug!("UdpReceiver::ack");
                let id: u64 = deserialize_payload(&payload, "id").unwrap_or(0);
                let msg_str: String = deserialize_payload(&payload, "message").unwrap_or_default();
                let sequence_number: u64 =
                    deserialize_payload(&payload, "sequence_number").unwrap_or(0);
                let _ = self.server.as_ref().unwrap().try_send(NeighborAck {
                    id,
                    message: msg_str,
                    sequence_number,
                });
            }
            "get_leader" => {
                info!("[UDP] get_leader recibido (split)");
                let _ = self.server.as_ref().unwrap().try_send(SendLeaderId {
                    addr: addr.to_string(),
                });
            }
            other => {
                debug!("UDPReceiver: mensaje desconocido {}", other);
            }
        }

        // volver a escuchar
        let _ = _ctx.address().try_send(Listen {});
    }
}

#[async_handler]
impl Handler<SendMessage> for UdpReceiver {
    type Result = ();

    /// Envia un mensaje UDP al address dado por msg.addr
    async fn handle(&mut self, msg: SendMessage, _ctx: &mut Context<Self>) {
        let socket = self.socket.clone();
        let socket = async move {
            if let Err(_err) = socket.send_to(msg.message.as_bytes(), msg.addr).await {
                error!("Falló el envio UDP");
            }
            socket
        }
        .await;
        self.socket = socket;
    }
}
