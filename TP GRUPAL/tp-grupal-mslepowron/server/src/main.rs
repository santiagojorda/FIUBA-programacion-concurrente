mod apps_info;
mod connection_listener;
mod messages;
mod server;
mod server_connection;
mod tcp_sender;
mod udp_receiver;
mod udp_sender;
use crate::messages::{Ping, SetLeader};
use crate::server::get_neighbor_id;
use crate::server_connection::ServerConnection;
use crate::tcp_sender::{TcpMessage, TcpSender};
use crate::udp_receiver::{Listen, SetServer, UdpReceiver};
use crate::udp_sender::UdpSender;
use ::colog::{self};
use actix::prelude::*;
use app_utils::constants::{LEAD_SERVER_ADDRESS, MAX_SERVERS, SERVER_UDP_PORT_RANGE};
use app_utils::utils::{id_to_tcp_address, serialize_message};
use colored::*;
use log::{error, info};
use serde_json::json;
use server::Server;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader, split};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::time::Instant;
use tokio_stream::wrappers::LinesStream;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let mut args = env::args().skip(1);
    let id: u64 = args
        .next()
        .expect("El server se levanta como: cargo run id bool_lider")
        .parse()
        .expect("ID entero");
    let is_leader: bool = args
        .next()
        .expect("Falta flag de lider")
        .parse()
        .expect("Booleano");

    let level = log::LevelFilter::Debug;
    colog::basic_builder().filter_level(level).init();

    if is_leader {
        let bind_addrress = id_to_tcp_address(id);
        println!("Bind address calculada: {}", bind_addrress);
        let listener = match TcpListener::bind(&bind_addrress).await {
            Ok(l) => l,
            Err(e) => {
                error!("Error bind {}: {}", bind_addrress, e);
                return Err(e);
            }
        };

        println!("Esperando conexiones en {}", bind_addrress);

        let server = Server {
            customers: HashMap::new(),
            customers_address: HashMap::new(),
            clients_connected: 0,
            delivery_workers: Vec::new(),
            delivery_workers_address: HashMap::new(),
            restaurants: HashMap::new(),
            delivery_workers_info: Vec::new(),
            leader_connection: None,
            disconnected_servers: Vec::new(),
            received_neighbor_ack: false,
            election_on_course: false,
            actual_neighbor_id: get_neighbor_id(id),
            sequence_number: 0,
            message_received_ack: HashMap::new(),
            udp_sockets_replicas: HashMap::new(),
            id,
            ids_count: 0,
            id_leader: id,
            im_leader: true,
            udp_sender: None,
            servers: HashMap::new(),
        };

        let server_addr = server.start();
        let server_addr_cloned = server_addr.clone();

        info!("Esperando conexiones en {}", LEAD_SERVER_ADDRESS);

        let udp_port = SERVER_UDP_PORT_RANGE + id;
        let udp_addr = format!("127.0.0.1:{}", udp_port);
        let sock = Arc::new(UdpSocket::bind(&udp_addr).await?);

        // Iniciar actores UDP
        let udp_recv = UdpReceiver {
            socket: sock.clone(),
            server: None,
        }
        .start();

        let _udp_send = UdpSender {
            socket: sock.clone(),
        }
        .start();

        // Inyectar server_addr luego de spawn
        udp_recv.do_send(SetServer {
            server_addr: server_addr.clone(),
        });
        udp_recv.do_send(Listen {});

        while let Ok((stream, addr)) = listener.accept().await {
            info!("{}", format!("[{:?}] Cliente conectado\n", addr).yellow());

            let _ = TcpSender::create(|ctx| {
                let (read, write_half) = split(stream);

                TcpSender::add_stream(LinesStream::new(BufReader::new(read).lines()), ctx);

                let write = Some(write_half);
                TcpSender {
                    write,
                    addr,
                    server_addr: server_addr_cloned.clone(),
                    handshake_sent: true,
                }
            });
        }
    } else {
        // --- REPLICA: UDP bind y conexión al lider ---

        // Socket UDP para election y consultas
        let udp_port = SERVER_UDP_PORT_RANGE + id;
        let udp_socket_addr = format!("127.0.0.1:{}", udp_port);

        let Ok(udp_skt) = UdpSocket::bind(udp_socket_addr.clone()).await else {
            error!("Error al bindear socket UDP en {}", udp_socket_addr);
            return Ok(());
        };

        let udp_socket = Arc::new(udp_skt);
        let udp_socket_sender = udp_socket.clone();

        let udp_receiver = UdpReceiver {
            socket: udp_socket,
            server: None,
        }
        .start();

        let udp_sender_addr = UdpSender {
            socket: udp_socket_sender,
        }
        .start();

        // Inyectar server_addr luego de spawn
        let server = Server {
            id,
            id_leader: 0,
            ids_count: 0,
            im_leader: false,
            customers: HashMap::new(),
            customers_address: HashMap::new(),
            clients_connected: 0,
            delivery_workers: Vec::new(),
            delivery_workers_address: HashMap::new(),
            restaurants: HashMap::new(),
            delivery_workers_info: Vec::new(),
            leader_connection: None,
            disconnected_servers: Vec::new(),
            received_neighbor_ack: false,
            election_on_course: false,
            actual_neighbor_id: 0,
            sequence_number: 0,
            message_received_ack: HashMap::new(),
            udp_sockets_replicas: HashMap::new(),
            udp_sender: Some(udp_sender_addr.clone()),
            servers: HashMap::new(),
        };

        let server_addr = server.start();

        // Configurar receptor UDP
        udp_receiver.do_send(SetServer {
            server_addr: server_addr.clone(),
        });

        udp_receiver.do_send(Listen {});

        info!("Iniciando réplica");

        // Conectarse al líder vía TCP
        let mut leader_id = 1;
        let mut leader_addr = id_to_tcp_address(leader_id);
        if leader_id == id {
            leader_addr = id_to_tcp_address(leader_id + 1);
        }

        let mut connection = TcpStream::connect(leader_addr).await;
        if connection.is_err() {
            // intentar con otros servidores (1..MAX_SERVERS)
            for server_id in (1..=MAX_SERVERS).rev() {
                if server_id == id {
                    continue;
                }
                connection = TcpStream::connect(id_to_tcp_address(server_id)).await;
                if connection.is_ok() {
                    leader_id = server_id;
                    info!("Conectado al líder: {}", leader_id);
                    break;
                } else {
                    error!(
                        "No se pudo conectar al líder {}: {}",
                        server_id,
                        connection.as_ref().err().unwrap()
                    );
                }
            }
        }

        let Ok(connection) = connection else {
            error!("No se pudo conectar a ningún líder, terminando réplica");
            return Ok(());
        };

        let (r, w) = split(connection);
        let server_connection = ServerConnection::create(|ctx| {
            ServerConnection::add_stream(LinesStream::new(BufReader::new(r).lines()), ctx);
            ServerConnection {
                addr: server_addr.clone(),
                write: Some(w),
                response_time: Instant::now(),
            }
        });

        // Informar al servidor líder de la réplica
        let serialized =
            serialize_message("new_replica", json!({"id": id, "socket": udp_socket_addr}));

        if let Err(e) = server_connection.try_send(TcpMessage::new(serialized)) {
            error!("Error al enviar mensaje de nueva réplica: {}", e);
            return Ok(());
        }

        if let Err(e) = server_addr.try_send(SetLeader {
            id_leader: leader_id,
            leader_connection: server_connection.clone(),
        }) {
            error!("Error al establecer líder: {}", e);
            return Ok(());
        }

        actix::clock::sleep(Duration::from_secs(1)).await;

        loop {
            let ping_result = server_addr.try_send(Ping {});
            if ping_result.is_err() {
                error!("Error al enviar el Ping: {}", ping_result.err().unwrap());
                actix::clock::sleep(Duration::from_secs(5)).await;
            }
        }
    }
    Ok(())
}
