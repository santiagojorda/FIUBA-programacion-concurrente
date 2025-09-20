use crate::messages::{Ping, StartElection, UpdateNetworkState};
use crate::server::Server;
use crate::tcp_sender::TcpMessage;
use actix::{Actor, Addr, AsyncContext, Context, Handler, StreamHandler, WrapFuture};
use actix_async_handler::async_handler;
use log::{debug, error};
use serde_json::{Value, from_value};
use tokio::time::Instant;
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
};

/// Representa una conexión TCP con otro servidor.
#[derive(Debug)]
pub struct ServerConnection {
    /// Address del server
    pub addr: Addr<Server>,
    /// Extremo de escritura del socket
    pub write: Option<WriteHalf<TcpStream>>,
    /// Tiempo de respuesta luego del cual se considera caido el server lider
    pub response_time: Instant,
}

impl Actor for ServerConnection {
    type Context = Context<Self>;
}

#[async_handler]
impl Handler<TcpMessage> for ServerConnection {
    type Result = ();

    fn handle(&mut self, msg: TcpMessage, _ctx: &mut Self::Context) -> Self::Result {
        debug!("ServerConnection::TcpMessage");

        if let Some(mut writer) = self.write.take() {
            let message = msg.0.clone();
            let server_addr = self.addr.clone();
            // creamos un future que intenta escribir y, en caso de error, dispara StartElection
            let future = async move {
                if let Err(e) = writer.write_all(message.as_bytes()).await {
                    error!("Error al escribir al lider: {}", e);
                    let _ = server_addr.try_send(StartElection {});
                }
                writer
            }
            .into_actor(self)
            .map(|writer, actor, _| {
                // devolvemos el writer al actor
                actor.write = Some(writer);
            });
            _ctx.spawn(future);
        } else {
            error!("No hay writer disponible para enviar el mensaje");
        }
    }
}

impl Handler<Ping> for ServerConnection {
    type Result = Result<(), String>;

    fn handle(&mut self, _msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
        debug!("ServerConnection::Ping");
        Ok(())
    }
}

impl StreamHandler<Result<String, std::io::Error>> for ServerConnection {
    fn handle(&mut self, read: Result<String, std::io::Error>, _ctx: &mut Self::Context) {
        debug!("ServerConnection::StreamHandler");

        if let Ok(line) = read {
            let line = line.trim();
            debug!(" Receive '{}' from Replica", line.trim());

            if line.starts_with('{') {
                match serde_json::from_str::<Value>(line) {
                    Ok(v) => {
                        if let (Some(title), Some(payload)) =
                            (v.get("title").and_then(Value::as_str), v.get("payload"))
                        {
                            if title == "pong" {
                                self.response_time = Instant::now();
                                let ids_count = payload["ids_count"].as_u64().unwrap_or(0);
                                let deliveries =
                                    from_value(payload["deliveries"].clone()).unwrap_or_default();
                                let customers =
                                    from_value(payload["customers"].clone()).unwrap_or_default();
                                let restaurants =
                                    from_value(payload["restaurants"].clone()).unwrap_or_default();
                                let udp_sockets_replicas =
                                    from_value(payload["udp_sockets_replicas"].clone())
                                        .unwrap_or_default();
                                let disconnected_servers =
                                    from_value(payload["disconnected_servers"].clone())
                                        .unwrap_or_default();

                                let update = UpdateNetworkState {
                                    ids_count,
                                    deliveries,
                                    customers,
                                    restaurants,
                                    udp_sockets_replicas,
                                    disconnected_servers,
                                };

                                if let Err(e) = self.addr.try_send(update) {
                                    error!("Error al enviar el mensaje: {}", e);
                                }
                                return;
                            } else {
                                error!("Mensaje no esperado: {}", title);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error al parsear JSON: {}", e);
                        return;
                    }
                }
            }
        } else {
            error!("Failed to read line {:?}", read);
        }
    }
}
