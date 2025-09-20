pub mod customer;
mod delivery_connection;
mod gateway_connection;
pub mod messages;
pub mod receiver;
pub mod sender;
use crate::customer::Customer;
use crate::delivery_connection::DeliveryConnection;
use crate::gateway_connection::GatewayConnection;
use crate::messages::{AddDeliveryConnection, RecoverData};
use crate::receiver::TcpReceiver;
use crate::sender::{TcpMessage, TcpSender};
use actix::prelude::*;
use app_utils::constants::GATEWAY_ADDRESS;
use app_utils::payment_type::PaymentType;
use app_utils::utils::connect_to_server;
use colog::{self};
use colored::*;
use log::{error, info};
use messages::{GetRestaurants, Login, SetCustomer, SetReceiver};
use serde_json::Value;
use std::time::Duration;
use std::{env, io};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader, split};
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tokio_stream::wrappers::LinesStream;

/// Configuración inicial del cliente.
struct CustomerConfig {
    name: String,
    position: (u64, u64),
    payment_type: PaymentType,
    recover_customer: bool,
}

fn parse_args() -> CustomerConfig {
    let args: Vec<String> = env::args().collect();

    if args.len() != 6 {
        error!(
            "Uso: cargo run -- \"NOMBRE\" POS_X POS_Y PaymentType (cash o card) recovery(true o false)"
        );
        std::process::exit(0);
    }

    let name = args[1].clone();

    let pos_x = args[2].parse::<u64>().unwrap_or_else(|_| {
        error!("La coordenada X debe ser un número entero positivo.");
        std::process::exit(0);
    });

    let pos_y = args[3].parse::<u64>().unwrap_or_else(|_| {
        error!("La coordenada Y debe ser un número entero positivo.");
        std::process::exit(0);
    });

    let payment_type = match args[4].to_lowercase().as_str() {
        "cash" => PaymentType::Cash,
        "card" => PaymentType::CreditCard,
        other => {
            error!(
                "PaymentType inválido: '{}'. Debe ser 'cash' o 'card'",
                other
            );
            std::process::exit(0);
        }
    };

    let recover_customer = args[5].parse::<bool>().unwrap_or_else(|_| {
        error!("se espera un valor 'true' o 'false para indicar que se quiere recuperar la informacion del cliente o no.");
        std::process::exit(0);
    }) ;

    CustomerConfig {
        name,
        position: (pos_x, pos_y),
        payment_type,
        recover_customer,
    }
}

#[actix_rt::main]
async fn main() -> Result<(), io::Error> {
    let level = log::LevelFilter::Info;
    colog::basic_builder().filter_level(level).init();

    //funcion: Obtener parametros de entrada
    let customer_config = parse_args();

    //conectar al servidor
    let Ok(stream) = connect_to_server().await else {
        info!(
            "{}",
            format!("No fue posible conectar al cliente al servidor.").red()
        );
        return Ok(());
    };

    actix::clock::sleep(Duration::from_millis(2000)).await;

    info!(
        "{}",
        format!("[CUSTOMER]: Cliente conectado. Presiona Ctrl-C para salir.").green()
    );

    let address = stream
        .local_addr()
        .expect("Error al obtener el socket del servidor");

    let (read_half, write_half) = split(stream);

    let sender = TcpSender::new(write_half, address).start();

    let customer_sender = sender.clone();

    //CONEXION AL GATEWAY
    let Ok(stream_gateway) = TcpStream::connect(GATEWAY_ADDRESS).await else {
        error!("Error al conectarse al gateway.");
        return Ok(());
    };

    let (read_gateway, write_half_gateway) = split(stream_gateway);

    let gateway = GatewayConnection::create(|ctx| {
        GatewayConnection::add_stream(LinesStream::new(BufReader::new(read_gateway).lines()), ctx);
        let write = Some(write_half_gateway);
        GatewayConnection {
            write,
            customer_addr: None,
        }
    });

    let delivery_listener = TcpListener::bind("127.0.0.1:0").await?;
    let delivery_socket_addr = delivery_listener
        .local_addr()
        .expect("Error al obtener el socket del cliente.");
    let delivery_socket = delivery_socket_addr.to_string();
    info!(
        "[CUSTOMER]: Cliente escuchando repartidores en el socket: {}",
        delivery_socket
    );

    let customer = Customer {
        id: 0, // El ID se asignará en el servidor
        name: customer_config.name.clone(),
        position: customer_config.position,
        chosen_restaurant: None,
        id_order: 0,
        payment_type: customer_config.payment_type,
        payment_authorized: false,
        payment_confirmed: false,
        tcp_sender: customer_sender.clone(),
        tcp_server_receiver: None,
        handshake_done: false,
        available_restaurants: Vec::new(),
        selected_restaurant_socket: None,
        restaurant_connections: Vec::new(),
        restaurant_sender: None,
        restaurant_receiver: None,
        gateway: gateway.clone(),
        waiting_gateway_auth: true,
        waiting_restaurant_auth: true,
        gateway_vote: None,
        restaurant_vote: None,
        recover_customer_data: customer_config.recover_customer,
        delivery_socket: delivery_socket.clone(),
        assigned_delivery_socket: None,
        delivery_connections: Vec::new(),
    }
    .start();

    if let Err(e) = gateway.try_send(SetCustomer {
        customer: customer.clone(),
    }) {
        error!("Error al enviar el mensaje SetCustomer: {}", e);
        return Ok(());
    }

    let server_receiver = TcpReceiver::create(|ctx| {
        TcpReceiver::add_stream(LinesStream::new(BufReader::new(read_half).lines()), ctx);
        TcpReceiver {
            client_addr: customer.clone(),
            addr: address,
        }
    });

    if let Err(e) = customer.try_send(SetReceiver {
        receiver: server_receiver,
    }) {
        error!("Error al enviar el mensaje SetReceiver: {}", e);
        return Ok(());
    }

    if customer_config.recover_customer {
        let mut customer_status = String::new();
        let customer_name = customer_config.name.clone();
        if let Ok(mut customer_file) =
            tokio::fs::File::open(format!("client_{customer_name}.json")).await
        {
            if customer_config.recover_customer {
                customer_file.read_to_string(&mut customer_status).await?;

                let customer_status: Value = serde_json::from_str(&customer_status)?;
                if let Err(e) = customer.try_send(RecoverData { customer_status }) {
                    error!("Error al enviar el mensaje: {}", e);
                };
            }
        } else {
            if let Ok(()) = customer.try_send(Login {
                name: customer_config.name.clone(),
            }) {
                actix::clock::sleep(Duration::from_millis(2000)).await;

                if let Err(e) = customer.try_send(GetRestaurants {}) {
                    error!("Error al enviar el mensaje: {}", e);
                }
            } else {
                error!("[CUSTOMER]: Error al enviar el mensaje de loggueo");
            }
        }
    } else {
        //LOGIN A LA APP: el customer le manda sus datos al servidor(nombre) y este inicia su sesion.
        if let Ok(()) = customer.try_send(Login {
            name: customer_config.name.clone(),
        }) {
            actix::clock::sleep(Duration::from_millis(2000)).await;

            //REQUEST DE LOS RESTUARANTES: el cliente le pide al servidor que le envie los restaurantes disponibles CON GetRestaurants

            if let Err(e) = customer.try_send(GetRestaurants {}) {
                error!("Error al enviar el mensaje: {}", e);
            }
            //socket para aceptar mensajes de deliveries
            {
                let cust_addr = customer.clone();
                actix::spawn(async move {
                    while let Ok((stream, peer)) = delivery_listener.accept().await {
                        // info!("[CUSTOMER] Repartidor conectado desde {:?}", peer);
                        let (read_half, write_half) = split(stream);
                        let delivery_connection = DeliveryConnection::create(|ctx| {
                            // agregamos la línea al actor para que reciba mensajes
                            DeliveryConnection::add_stream(
                                LinesStream::new(BufReader::new(read_half).lines()),
                                ctx,
                            );
                            DeliveryConnection {
                                write: Some(write_half),
                                client_addr: cust_addr.clone(),
                                socket_addr: delivery_socket_addr,
                            }
                        });
                        if let Err(e) = cust_addr.try_send(AddDeliveryConnection {
                            addr: delivery_connection.clone(),
                        }) {
                            error!("No se pudo guardar la conexión del delivery: {}", e);
                        }
                    }
                });
            }
        } else {
            error!("[CUSTOMER]: Error al enviar el mensaje de loggueo");
        }
    }
    shutdown(sender).await;

    Ok(())
}

/// Espera a Ctrl-C, envía el mensaje de cierre y lo muestra por consola.
async fn shutdown(sender: Addr<TcpSender>) {
    signal::ctrl_c().await.expect("Error al capturar Ctrl-C");

    sender.do_send(TcpMessage::new("[CUSTOMER]: Cerrando conexión...")); //esto deberia tener un handle en el server
    info!("[CUSTOMER]: Mensaje de desconexión enviado, cerrando.");
}
