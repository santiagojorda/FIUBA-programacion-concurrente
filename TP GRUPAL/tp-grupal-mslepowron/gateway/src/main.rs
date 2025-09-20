mod customer_sender;
mod gateway;
mod messages;
use crate::customer_sender::TcpSender;
use crate::gateway::Gateway;
use crate::messages::NewCustomer;
use actix::prelude::*;
use app_utils::constants::GATEWAY_ADDRESS;
use colored::*;
use log::{error, info};
use std::collections::HashMap;

use tokio::io::{AsyncBufReadExt, BufReader, split};
use tokio::net::TcpListener;
use tokio_stream::wrappers::LinesStream;

#[actix_rt::main]
async fn main() {
    let level = log::LevelFilter::Info;
    colog::basic_builder().filter_level(level).init();

    info!(
        "{}",
        format!("[GATEWAY]: Conectado. Presiona Ctrl-C para salir.").yellow()
    );

    let Ok(listener) = TcpListener::bind(GATEWAY_ADDRESS).await else {
        error!(
            "{}",
            format!("Error al bindear socket en address {}", GATEWAY_ADDRESS).red()
        );
        return;
    };

    info!("{}", format!("[GATEWAY]: esperando comensales 💰"));

    let gateway = Gateway {
        customer_connections: Vec::new(),
        customers: HashMap::new(),
        customer_id_count: 0,
    };
    let gateway_address = gateway.start();

    while let Ok((stream, addr)) = listener.accept().await {
        info!("{}", format!("[{:?}] Cliente conectado\n", addr).green());

        let addr = TcpSender::create(|ctx| {
            let (read, write_half) = split(stream);
            TcpSender::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
            let write = Some(write_half);
            TcpSender { write, addr }
        });

        if let Err(e) = gateway_address.try_send(NewCustomer { addr }) {
            error!("Error al enviar el mensaje: {}", e);
        }
    }
}
