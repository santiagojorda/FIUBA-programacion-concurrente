use crate::admin_actor::admin::Admin;
use crate::admin_actor::admin_to_coord::MakeTrip;
use crate::coordinator_actor::coordinator_messages::HandleTrip;
use crate::elections::election_messages::GetCoordAddr;
use crate::storage_actor::storage_messages::GetNearestDriver;
use crate::utils::consts::MAX_RETRIES;
use crate::utils::logs::log_trips;
use actix::prelude::*;
use actix::Message;
use common::tcp_sender::TcpMessage;
use std::net::SocketAddr;
use tokio::io::{split, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[derive(Message, Debug)]
#[rtype(result = "()")]
/// Message to request a trip from a Passenger to the Admin
pub struct FindNearestDriver {
    pub passenger_addr: SocketAddr,
}

impl Handler<HandleTrip> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: HandleTrip, ctx: &mut Self::Context) -> Self::Result {
        log_trips("| ----- Handle Trip ----- |".to_string());

        let actor_addr = ctx.address();

        Box::pin(
            async move {
                let find_msg = FindNearestDriver {
                    passenger_addr: msg.passenger_id_ht,
                };

                actor_addr
                    .try_send(find_msg)
                    .expect("Failed to send FindNearestDriver");
            }
            .into_actor(self),
        )
    }
}

impl Handler<FindNearestDriver> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: FindNearestDriver, ctx: &mut Self::Context) -> Self::Result {
        let coord_elect_clone = self.coordinator_election.clone();
        let current_passenger = _msg.passenger_addr;
        let storage_actor = self.storage_addr.clone();
        let self_addr = ctx.address();

        Box::pin(
            async move {
                log_trips("| ----- Find Nearest Driver ----- |".to_string());

                let mut make_trip = MakeTrip {
                    passenger_id_mt: current_passenger,
                    driver_id_mt: SocketAddr::new([0, 0, 0, 0].into(), 0),
                };

                if let Some(nearest_driver) = storage_actor
                    .send(GetNearestDriver {})
                    .await
                    .expect("Failed to get nearest driver")
                {
                    log_trips(format!("Found a Driver!: {:?}", nearest_driver));
                    make_trip = MakeTrip {
                        passenger_id_mt: current_passenger,
                        driver_id_mt: nearest_driver,
                    };
                }

                // Send to coordinator
                let coord_addr = coord_elect_clone
                    .send(GetCoordAddr {})
                    .await
                    .expect("Failed to get coordinator address");

                if let Some(coord_addr) = coord_addr {
                    send_trip_to_coordinator(make_trip, coord_addr, self_addr).await;
                } else {
                    log_trips("Coordinator address not found".to_string());
                }
            }
            .into_actor(self),
        )
    }
}

async fn send_trip_to_coordinator(
    make_trip: MakeTrip,
    coordinator_addr: SocketAddr,
    admin_addr: Addr<Admin>,
) {
    log_trips(format!(
        "| ----- Send Trip to Coordinator [{:?}] ----- |",
        coordinator_addr
    ));
    let mut got_res = false;
    let mut got_ack = false;

    for _ in 0..MAX_RETRIES {
        let con = TcpStream::connect(coordinator_addr).await;
        match con {
            Ok(s) => {
                let stream = Some(s);
                let (reader, mut writer) = match stream {
                    Some(stream) => split(stream),
                    None => panic!("Stream is None"),
                };

                if let Ok(serialized) = serde_json::to_string(&make_trip) {
                    let serialized = format!("{}\n", serialized);
                    let msg = TcpMessage(serialized);
                    match writer.write_all(msg.0.as_bytes()).await {
                        Ok(_) => (),
                        Err(e) => println!("Error writing make_trip to writer: {}", e),
                    }
                } else {
                    eprintln!("Error serializing make_trip");
                }

                let mut reader = BufReader::new(reader);
                let mut line = String::new();

                match timeout(Duration::from_secs(3), reader.read_line(&mut line)).await {
                    Ok(Ok(_)) => {
                        if line.trim() == "Ack" {
                            got_ack = true;
                        }
                        got_res = true;
                    }
                    Ok(Err(_)) => {
                        println!("Failed to read line");
                    }
                    Err(_) => {
                        log_trips("Timeout".to_string());
                        got_res = false;
                    }
                }
                break;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        }
    }

    if !got_res {
        log_trips("Failed to send trip to coordinator".to_string());
        return;
    }

    if got_ack {
        log_trips("Sent trip to coordinator".to_string());
    } else {
        admin_addr
            .try_send(HandleTrip {
                passenger_id_ht: make_trip.passenger_id_mt,
            })
            .expect("Failed to self-send FindNearestDriver");
    }
}
