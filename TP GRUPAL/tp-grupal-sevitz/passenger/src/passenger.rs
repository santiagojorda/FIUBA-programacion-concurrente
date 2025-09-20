use common::messages::{FinishTrip, RejectTrip, RequestTrip, StartTrip, WhoIsCoordinatorResponse};
use common::tcp_sender::TcpMessage;
use common::utils::get_rand_f32_tuple;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{split, AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Passenger struct.
/// Models a passenger in the system.
pub struct Passenger {
    servers: Vec<SocketAddr>,
    reader: Lines<BufReader<OwnedReadHalf>>,
    writer: OwnedWriteHalf,
}

impl Passenger {
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
        self.request_trip().await;

        loop {
            tokio::select! {
                result = self.reader.next_line() => {
                    match result {
                        Ok(Some(message)) => {
                            self.handle_server_message(message).await;
                        }
                        Ok(None) => {
                            println!("[PASSENGER] Server closed the connection. Exiting...");
                            break;
                        }
                        Err(e) => {
                            eprintln!("[PASSENGER] Error reading from server: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }
    async fn request_trip(&mut self) {
        println!("[PASSENGER] Requesting a trip...");
        let request_trip = RequestTrip {
            origin: get_rand_f32_tuple(),
            destination: get_rand_f32_tuple(),
        };
        if let Ok(serialized) = serde_json::to_string(&request_trip) {
            if let Err(e) = self
                .writer
                .write_all(format!("{}\n", serialized).as_bytes())
                .await
            {
                eprintln!("[PASSENGER] Failed to send RequestTrip: {}", e);
            }
        }
    }
    async fn handle_server_message(&mut self, message: String) {
        if let Ok(start_trip) = serde_json::from_str::<StartTrip>(&message) {
            self.start_trip(start_trip).await;
        } else if let Ok(reject_trip) = serde_json::from_str::<RejectTrip>(&message) {
            println!("[PASSENGER] Trip rejected: [{:?}]", reject_trip.response);
        } else {
            println!("[PASSENGER] Unknown message: {}", message);
        }
    }

    async fn start_trip(&mut self, msg: StartTrip) {
        let position = msg.origin;
        let destination = msg.destination;

        println!(
            "[PASSENGER] Trip started from ({}, {}) to ({}, {})",
            position.0, position.1, destination.0, destination.1
        );
        let distance =
            ((position.0 - destination.0).powi(2) + (position.1 - destination.1).powi(2)).sqrt();
        tokio::time::sleep(tokio::time::Duration::from_secs_f32(distance)).await;
        println!("[PASSENGER] Trip finished");
        let trip_finished = FinishTrip {
            passenger_id_ft: msg.passenger_id_st,
            driver_id_ft: msg.driver_id_st,
            destination_pos: (destination.0, destination.1),
        };

        if let Ok(trip_finished_ser) = serde_json::to_string(&trip_finished) {
            if (self
                .writer
                .write_all(format!("{}\n", trip_finished_ser).as_bytes())
                .await).is_err()
            {
                println!("[PASSENGER] Failed to send FinishTrip. Attempting to reconnect...");
                if !self.attempt_reconnect(trip_finished_ser).await {
                    eprintln!("[PASSENGER] Unable to reconnect to any server.");
                }
                return;
            }

            let ack_future = self.reader.next_line();
            match tokio::time::timeout(tokio::time::Duration::from_secs(2), ack_future).await {
                Ok(Ok(Some(ack_message))) => {
                    if ack_message.trim() == "ACK" {
                        println!(
                            "[PASSENGER] Received ACK from server. Trip successfully finished."
                        );
                    } else {
                        println!(
                            "[PASSENGER] Unexpected message from server: {}",
                            ack_message
                        );
                    }
                }
                Ok(Ok(None)) | Ok(Err(_)) | Err(_) => {
                    println!("[PASSENGER] Error or timeout while waiting for ACK. Attempting to reconnect...");
                    if !self.attempt_reconnect(trip_finished_ser).await {
                        eprintln!("[PASSENGER] Unable to reconnect to any server.");
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
                eprintln!("[PASSENGER] Failed to send FinishTrip: {}", e);
            } else {
                println!("[PASSENGER] Successfully reconnected and sent FinishTrip");
                return true;
            }
        }
        false
    }
}

async fn connect_to_coordinator(servers: Vec<SocketAddr>) -> Option<TcpStream> {
    let mut coord_addr: Option<SocketAddr> = None;
    println!("[PASSENGER] Asking For Coordinator...");

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
                println!("[PASSENGER] Could not connect to coordinator at {}", server);
            }
        }

        if coord_addr.is_some() {
            break;
        }
    }

    if let Some(coord_ip) = coord_addr {
        println!("[PASSENGER] Found Coordinator at {}", coord_ip);
        let stream = match TcpStream::connect(coord_ip).await {
            Ok(stream) => {
                println!("[PASSENGER] Connected to Coordinator");
                Some(stream)
            }
            Err(_) => None,
        };

        return stream;
    }

    None
}
