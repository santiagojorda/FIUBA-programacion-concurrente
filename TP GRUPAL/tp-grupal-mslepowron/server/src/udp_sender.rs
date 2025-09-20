use std::sync::Arc;

use actix::{Actor, Context, Handler, Message};
use actix_async_handler::async_handler;
use log::{debug, error};
use tokio::net::UdpSocket;

/// Representa un actor que envia mensajes UDP.
pub struct UdpSender {
    pub socket: Arc<UdpSocket>,
}

impl Actor for UdpSender {
    type Context = Context<Self>;
}

/// Representa un mensaje UDP, con el contenido y el destino.
#[derive(Message)]
#[rtype(result = "()")]
pub struct UdpMessage {
    pub message: String,
    pub dst: String,
}

#[async_handler]
impl Handler<UdpMessage> for UdpSender {
    type Result = ();

    /// Envia un mensaje UDP.
    async fn handle(&mut self, msg: UdpMessage, _ctx: &mut Context<Self>) {
        debug!("Enviando {} a {}", msg.message, msg.dst);

        let socket = self.socket.clone();
        self.socket = async move {
            if let Err(err) = socket.send_to(msg.message.as_bytes(), msg.dst).await {
                error!("Falló el envio del mensaje {}. Error: {}", msg.message, err)
            }
            socket
        }
        .await;
    }
}
