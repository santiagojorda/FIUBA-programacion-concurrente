use std::net::SocketAddr;

use actix::prelude::*;
use actix_async_handler::async_handler;
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
}

impl TcpSender {
    pub fn new(write: WriteHalf<TcpStream>, addr: SocketAddr) -> Self {
        TcpSender {
            write: Some(write),
            addr,
        }
    }
}

impl Actor for TcpSender {
    type Context = Context<Self>;
}

///el Sender de un restaurant se usa para comunicarse con el servidor.
/// su tarea principal es convertir mensajes de tipo String en TcpMessage, serialziarlos y enviarlos al servidor.
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
