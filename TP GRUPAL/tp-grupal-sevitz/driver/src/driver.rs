use common::messages::{
    CanAcceptTrip, CanAcceptTripResponse, DriverPosition, FinishTrip, StartTrip,
    WhoIsCoordinatorResponse,
};
use common::tcp_sender::TcpMessage;
use common::utils::get_rand_f32_tuple;
use rand::Rng;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{split, AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

/// Driver struct.
/// Models a driver in the system.
pub struct Driver {
    servers: Vec<SocketAddr>,
    reader: Lines<BufReader<OwnedReadHalf>>,
    writer: OwnedWriteHalf,
}

impl Driver {
    pub async fn new(servers: Vec<SocketAddr>) -> Self {
        let tcp_stream: Option<TcpStream> = connect_to_coordinator(servers.clone()).await;

        if let Some(stream) = tcp_stream {
            let (rx, wx) = stream.into_split();
            Self {
                servers,
                reader: BufReader::new(rx).lines(),
                writer: wx,
            }
        } else {
            panic!("Unable to connect to any server.");
        }
    }

    pub async fn run(&mut self) {
        self.send_position().await;
        loop {
            tokio::select! {
                result = self.reader.next_line() => {
                    match result {
                        Ok(Some(message)) => {
                            self.handle_server_message(message).await;
                        }
                        Ok(None) => {
                            println!("[DRIVER] Server closed the connection.");
                            break;
                        }
                        Err(e) => {
                            eprintln!("[DRIVER] Error reading from server: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn handle_server_message(&mut self, message: String) {
        if let Ok(request) = serde_json::from_str::<CanAcceptTrip>(&message) {
            self.handle_can_accept_trip(request).await;
        } else if let Ok(start_trip) = serde_json::from_str::<StartTrip>(&message) {
            self.handle_start_trip(start_trip).await;
        } else {
            eprintln!("[DRIVER] Unknown message: {}", message);
        }
    }

    async fn handle_can_accept_trip(&mut self, msg: CanAcceptTrip) {
        println!("[DRIVER] Handling trip request...");

        let mut rng = rand::thread_rng();
        let is_accepted = rng.gen_bool(0.8);

        println!(
            "[DRIVER] Trip {}",
            if is_accepted { "accepted" } else { "rejected" }
        );

        let response = CanAcceptTripResponse {
            passenger_id_car: msg.passenger_id_ca,
            is_accepted,
        };

        if let Ok(serialized) = serde_json::to_string(&response) {
            if let Err(err) = self
                .writer
                .write_all(format!("{}\n", serialized).as_bytes())
                .await
            {
                eprintln!("[DRIVER] Failed to send response: {}", err);
            }
        }
    }

    async fn send_position(&mut self) {
        println!("[DRIVER] Sending position...");
        let position = DriverPosition {
            position: get_rand_f32_tuple(),
        };
        println!(
            "[DRIVER] Position sent: ({}, {})",
            position.position.0, position.position.1
        );
        if let Ok(serialized) = serde_json::to_string(&position) {
            if let Err(err) = self
                .writer
                .write_all(format!("{}\n", serialized).as_bytes())
                .await
            {
                eprintln!("[DRIVER] Failed to send position: {}", err);
            }
        }
    }

    async fn handle_start_trip(&mut self, msg: StartTrip) {
        let position = msg.origin;
        let destination = msg.destination;

        println!(
            "[DRIVER] Trip started from ({}, {}) to ({}, {})",
            position.0, position.1, destination.0, destination.1
        );

        let distance =
            ((position.0 - destination.0).powi(2) + (position.1 - destination.1).powi(2)).sqrt();
        let delay = rand::thread_rng().gen_range(1.0..=2.0);

        sleep(Duration::from_secs_f32(distance + delay)).await;

        println!("[DRIVER] Trip finished");

        let trip_finished = FinishTrip {
            passenger_id_ft: msg.passenger_id_st,
            driver_id_ft: msg.driver_id_st,
            destination_pos: (destination.0, destination.1),
        };

        if let Ok(trip_finished_ser) = serde_json::to_string(&trip_finished) {
            if let Err(err) = self
                .writer
                .write_all(format!("{}\n", trip_finished_ser).as_bytes())
                .await
            {
                eprintln!("[DRIVER] Failed to send FinishTrip: {}", err);
            }
            let ack_future = self.reader.next_line();
            match tokio::time::timeout(Duration::from_secs(2), ack_future).await {
                Ok(Ok(Some(ack_message))) => {
                    if ack_message.trim() == "ACK" {
                        println!("[DRIVER] Received ACK from server. Trip successfully finished.");
                    } else {
                        println!("[DRIVER] Unexpected message from server: {}", ack_message);
                    }
                }
                Ok(Ok(None)) | Ok(Err(_)) | Err(_) => {
                    println!("[DRIVER] Error or timeout while waiting for ACK. Attempting to reconnect...");
                    if !self.attempt_reconnect(trip_finished_ser).await {
                        eprintln!("[DRIVER] Unable to reconnect to any server.");
                    }
                }
            }
        }
    }

    async fn attempt_reconnect(&mut self, message: String) -> bool {
        let coord = connect_to_coordinator(self.servers.clone()).await;
        if let Some(stream) = coord {
            let (reader, writer) = stream.into_split();
            self.reader = BufReader::new(reader).lines();
            self.writer = writer;

            if let Err(e) = self
                .writer
                .write_all(format!("{}\n", message).as_bytes())
                .await
            {
                eprintln!("[DRIVER] Failed to send FinishTrip: {}", e);
            } else {
                println!("[DRIVER] Successfully reconnected and sent FinishTrip");
                return true;
            }
        }
        false
    }
}

async fn connect_to_coordinator(servers: Vec<SocketAddr>) -> Option<TcpStream> {
    let mut coord_addr: Option<SocketAddr> = None;
    println!("[DRIVER] Asking For Coordinator...");

    for server in servers {
        match TcpStream::connect(server).await {
            Ok(stream) => {
                let (reader, mut writer) = split(stream);
                let msg = TcpMessage("WhoIsCoordinator\n".to_string());
                if let Err(e) = writer.write_all(msg.0.as_bytes()).await {
                    eprintln!("Error writing CoordinatorElection message: {}", e);
                }
                let mut reader = BufReader::new(reader);
                let mut line = String::new();

                match timeout(Duration::from_secs(3), reader.read_line(&mut line)).await {
                    Ok(Ok(_)) => {
                        if let Ok(who_is_coord_msg) =
                            serde_json::from_str::<WhoIsCoordinatorResponse>(&line)
                        {
                            coord_addr = Some(who_is_coord_msg.coord_id);
                            break;
                        } else if line.trim() == "Ack" {
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        println!("Failed to read line: {:?}", e);
                    }
                    Err(_) => {
                        println!("Timeout reading line");
                    }
                }
                break;
            }
            Err(_) => {
                println!("[DRIVER] Could not connect to coordinator at {}", server);
            }
        }

        if coord_addr.is_some() {
            break;
        }
    }

    if let Some(coord_ip) = coord_addr {
        println!("[DRIVER] Found Coordinator at {}", coord_ip);
        let stream = match TcpStream::connect(coord_ip).await {
            Ok(stream) => {
                println!("[DRIVER] Connected to Coordinator");
                Some(stream)
            }
            Err(_) => None,
        };

        return stream;
    }

    None
}
