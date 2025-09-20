pub mod delivery;
pub mod messages;
pub mod receiver;
pub mod restaurant_connection;
pub mod sender;
use crate::delivery::DeliveryWorker;
use crate::messages::{NewConnection, RegisterDelivery};
use crate::receiver::TcpReceiver;
use crate::restaurant_connection::RestaurantConnection;
use crate::sender::{TcpMessage, TcpSender};
use actix::prelude::*;
use app_utils::utils::connect_to_server;
use colog;
use colored::*;
use log::{error, info};
use messages::{LoginDelivery, SetReceiver};
use std::net::SocketAddr;
use std::{env, io::Result};
use tokio::io::{AsyncBufReadExt, BufReader, split};
use tokio::net::TcpListener;
use tokio::signal;
use tokio_stream::wrappers::LinesStream;

/// Configuración inicial del repartidor.
struct DeliveryConfig {
    name: String,
    position: (u64, u64),
}

fn parse_args() -> DeliveryConfig {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        error!("Uso: cargo run -- \"NOMBRE\" POS_X POS_Y");
        std::process::exit(1);
    }

    let name = args[1].clone();

    let pos_x = args[2].parse::<u64>().unwrap_or_else(|_| {
        error!("La coordenada X debe ser un número entero positivo.");
        std::process::exit(1);
    });

    let pos_y = args[3].parse::<u64>().unwrap_or_else(|_| {
        error!("La coordenada Y debe ser un número entero positivo.");
        std::process::exit(1);
    });

    DeliveryConfig {
        name,
        position: (pos_x, pos_y),
    }
}

#[actix_rt::main]
async fn main() -> Result<()> {
    let level = log::LevelFilter::Info;
    colog::basic_builder().filter_level(level).init();

    let delivery_config = parse_args();

    let Ok(stream) = connect_to_server().await else {
        info!("No fue posible conectar el delivery al servidor.");
        return Ok(());
    };

    info!(
        "{}",
        "[DELIVERY]: Delivery conectado. Presiona Ctrl-C para salir.".magenta()
    );

    let address = stream
        .local_addr()
        .expect("Error al obtener el socket del servidor");

    let (read_half, write_half) = split(stream);
    let sender = TcpSender::new(write_half, address).start();
    let delivery_sender = sender.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let delivery_socket = listener
        .local_addr()
        .expect("Error al obtener el socket del repartidor.")
        .to_string();

    let delivery = DeliveryWorker {
        id: 0,
        name: delivery_config.name.clone(),
        worker_position: delivery_config.position,
        available: true,
        tcp_sender: delivery_sender.clone(),
        tcp_server_receiver: None,
        handshake_done: false,
        connections: Vec::new(),
        delivery_socket: delivery_socket.clone(),
        restaurant_position: None,
        customer_position: None,
        customer_socket: None,
    }
    .start();

    let server_receiver = TcpReceiver::create(|ctx| {
        TcpReceiver::add_stream(LinesStream::new(BufReader::new(read_half).lines()), ctx);
        TcpReceiver {
            client_addr: delivery.clone(),
            addr: address,
        }
    });

    if let Err(e) = delivery.try_send(SetReceiver {
        receiver: server_receiver,
    }) {
        error!("Error al enviar el mensaje SetReceiver: {}", e);
        return Ok(());
    }

    //LOGIN A LA APP: el delivery le manda sus datos al servidor(nombre) y este inicia su sesion.
    match delivery.try_send(LoginDelivery {
        name: delivery_config.name.clone(),
        position: delivery_config.position,
    }) {
        Ok(_) => (),
        Err(e) => {
            error!("[DELIVERY]: Error al enviar el mensaje de loggueo: {}", e);
        }
    }

    let restaurant_listener = TcpListener::bind("127.0.0.1:0").await?;
    let restaurant_addr: SocketAddr = restaurant_listener.local_addr()?;
    let restaurant_socket = restaurant_addr.to_string();
    info!(
        "[DELIVERY]: Escuchando restaurantes en {}",
        restaurant_socket
    );

    // registro el socket del delivery en el servidor
    delivery
        .try_send(RegisterDelivery {
            position: delivery_config.position,
            delivery_socket: restaurant_socket.clone(),
        })
        .map_err(|e| error!("Error enviando RegisterDelivery: {}", e))
        .ok();

    info!(
        "[DELIVERY]: Escuchando restaurantes en puerto {}",
        restaurant_socket
    );

    // escucho por conexiones de restaurantes en otro puerto
    while let Ok((stream, addr)) = restaurant_listener.accept().await {
        info!("{}", format!("[{:?}] Restaurante conectado\n", addr).blue());

        let (read, write_half) = split(stream);
        let delivery_addr = delivery.clone();
        let connection = RestaurantConnection::create(|ctx| {
            RestaurantConnection::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
            RestaurantConnection {
                delivery_addr: delivery_addr.clone(),
                addr,
                write: Some(write_half),
            }
        });
        if let Err(e) = delivery.try_send(NewConnection { addr: connection }) {
            error!("Error al enviar el mensaje: {}", e);
        };
    }

    shutdown(sender).await;
    Ok(())
}

async fn shutdown(sender: Addr<TcpSender>) {
    signal::ctrl_c().await.expect("Error al capturar Ctrl-C");

    sender.do_send(TcpMessage::new("[DELIVERY]: Cerrando conexión..."));
    println!("[DELIVERY]: Mensaje de desconexión enviado, cerrando.");
}
