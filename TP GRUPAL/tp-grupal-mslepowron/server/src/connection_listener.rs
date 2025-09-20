use crate::{server::Server, tcp_sender::TcpSender, udp_receiver::Listen};
use actix::{Actor, Addr, Context, Handler, StreamHandler};
use actix_async_handler::async_handler;
use colored::Colorize;
use log::{debug, info};
use tokio::{
    io::{AsyncBufReadExt, BufReader, split},
    net::TcpListener,
};
use tokio_stream::wrappers::LinesStream;

/// Representa un actor que escucha por conexiones TCP.
pub struct ConnectionListener {
    /// Socket en el cual escucha conexiones
    pub socket: String,
    /// Address del actor Server
    pub server_addr: Addr<Server>,
}

impl Actor for ConnectionListener {
    type Context = Context<Self>;
}

#[allow(clippy::unused_unit)]
#[async_handler]
impl Handler<Listen> for ConnectionListener {
    type Result = ();

    /// Escucha por conexiones TCP.
    async fn handle(&mut self, _msg: Listen, _ctx: &mut Self::Context) -> Self::Result {
        debug!("ConnectionListener::Listen\n");
        let address = self.socket.clone();
        let listener = async move { TcpListener::bind(address).await }.await;
        if let Ok(listener) = listener {
            connection_loop(listener, self.server_addr.clone()).await
        }
    }
}

/// Escucha por conexiones TCP, y por cada una, crea un actor TcpSender, el cual
/// se comunica con el actor Server cada vez que recibe un mensaje TCP.
async fn connection_loop(listener: TcpListener, server_addr: Addr<Server>) {
    debug!("Connection loop");
    while let Ok((stream, addr)) = listener.accept().await {
        info!("{}", format!("[{:?}] Nueva conexion\n", addr).green());

        let _addr = TcpSender::create(|ctx| {
            let (read, write_half) = split(stream);
            TcpSender::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
            let write = Some(write_half);
            TcpSender {
                write,
                addr,
                server_addr: server_addr.clone(),
                handshake_sent: false,
            }
        });
    }
}
