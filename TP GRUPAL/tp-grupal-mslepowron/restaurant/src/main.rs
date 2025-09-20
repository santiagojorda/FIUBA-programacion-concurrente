mod customer_connection;
mod delivery_connection;
mod messages;
pub mod receiver;
mod restaurant;
pub mod sender;
use crate::messages::{NewCustomerConnection, RegisterRestaurant, SetReceiver};
use crate::receiver::TcpReceiver;
use crate::restaurant::Restaurant;
use crate::sender::{TcpMessage, TcpSender};
use actix::StreamHandler;
use actix::prelude::*;
use app_utils::utils::connect_to_server;
use colored::Colorize;
use customer_connection::CustomerConnection;
use log::{error, info};
use std::env;
use std::io::Result;
use tokio::io::{AsyncBufReadExt, BufReader, split};
use tokio::net::TcpListener;
use tokio::signal;
use tokio_stream::wrappers::LinesStream;

struct RestaurantConfig {
    name: String,
    position: (u64, u64),
}

fn parse_args() -> RestaurantConfig {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        error!("Uso: cargo run -- \"NOMBRE RESTAURANTE \" POS_X POS_Y");
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

    RestaurantConfig {
        name,
        position: (pos_x, pos_y),
    }
}

#[actix_rt::main]
async fn main() -> Result<()> {
    let level = log::LevelFilter::Info;
    colog::basic_builder().filter_level(level).init();

    //funcion: Obtener parametros de entrada
    let restaurant_config = parse_args();

    // Conectar al servidor
    let Ok(stream) = connect_to_server().await else {
        info!(
            "{}",
            format!("No fue posible conectar al cliente al servidor.").red()
        );
        return Ok(());
    };

    info!(
        "{}",
        format!(
            "[RESTAURANT]: {} Restaurante conectado. Presiona Ctrl-C para salir.",
            restaurant_config.name.clone()
        )
        .blue()
    );

    //Socket para comunicarse con customers
    let restaurant_socket: String = "127.0.0.1:0".into();
    let listener = TcpListener::bind(restaurant_socket.clone()).await?;
    let restaurant_socket = listener
        .local_addr()
        .expect("Error al obtener el socket del restaurnate.")
        .to_string();

    let address = stream
        .local_addr()
        .expect("Error al obtener el socket del servidor");

    let (read_half, write_half) = split(stream);

    let sender = TcpSender::new(write_half, address).start();

    let restaurant_sender = sender.clone();

    let restaurant_client = Restaurant {
        name: restaurant_config.name.clone(),
        restaurant_id: 0, // El ID se asignará más tarde por el servidor
        position: (
            restaurant_config.position.0 as u64,
            restaurant_config.position.1 as u64,
        ),
        orders_queue: Vec::new(),
        customers_connected: Vec::new(),
        tcp_sender: restaurant_sender,
        tcp_server_receiver: None,
        handshake_done: false,
        restaurant_socket: restaurant_socket.clone(),
        pending_deliveries: Vec::new(),
        has_deliveries: false,
        current_order_id: None,
        current_customer_socket: None,
        current_customer_position: None,
        next_order_id: 1,
    }
    .start();

    let receiver = TcpReceiver::create(|ctx| {
        TcpReceiver::add_stream(LinesStream::new(BufReader::new(read_half).lines()), ctx);
        TcpReceiver {
            addr: address,
            client_addr: restaurant_client.clone(),
        }
    });

    if let Err(e) = restaurant_client.try_send(SetReceiver { receiver }) {
        error!("Error al enviar el mensaje: {}", e);
    }

    //envio un mensaje al servidor con info para que se registre el restaurante
    match restaurant_client.try_send(RegisterRestaurant {
        socket: restaurant_socket,
        name: restaurant_config.name,
        position: restaurant_config.position,
    }) {
        Ok(_) => (),
        Err(e) => {
            error!("[CUSTOMER]: Error al enviar el mensaje de loggueo: {}", e);
        }
    }

    //Escucho por conexiones Tcp de Customer que quieran venir a hacer pedidos.
    while let Ok((stream, addr)) = listener.accept().await {
        info!("{}", format!("[{:?}] Comensal conectado\n", addr).green());

        let (read, write_half) = split(stream);
        let client_addr = restaurant_client.clone();
        let connection = CustomerConnection::create(|ctx| {
            CustomerConnection::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);
            CustomerConnection {
                write: Some(write_half),
                addr: addr,
                restaurant_address: client_addr,
            }
        });
        if let Err(e) = restaurant_client.try_send(NewCustomerConnection { addr: connection }) {
            error!("Error al enviar el mensaje: {}", e);
        };
    }
    shutdown(sender).await;

    Ok(())
}

/// Espera a Ctrl-C, envía el mensaje de cierre y lo muestra por consola.
async fn shutdown(sender: Addr<TcpSender>) {
    signal::ctrl_c().await.expect("Error al capturar Ctrl-C");

    sender.do_send(TcpMessage::new("[RESTAURANT]: Cerrando conexión..."));
    println!("[RESTAURANT]: Mensaje de desconexión enviado, cerrando.");
}
