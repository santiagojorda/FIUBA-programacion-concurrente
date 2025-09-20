use super::admin_to_coord::MakeTrip;
use crate::admin_actor::admin_to_storage::{MakeUpdateDriver, MakeUpdatePassenger};
use crate::admin_actor::ping::{spawn_ping_task, WhoIsCoordinator};
use crate::admin_actor::reaper::spawn_reaper_task;
use crate::coordinator_actor::coordinator::Coordinator;
use crate::coordinator_actor::coordinator_messages::{HandleTrip, UpdateDrivers, UpdatePassengers};
use crate::elections::election::CoordinatorElection;
use crate::elections::election_messages::{CoordinatorMessage, ElectionMessage, PingMessage};
use crate::storage_actor::storage::Storage;
use crate::utils::admin_errors::AdminError;
use crate::utils::logs::{log_elections, log_trips};
use actix::prelude::*;
use common::messages::{CanAcceptTripResponse, DriverPosition, FinishTrip, RequestTrip};
use common::tcp_sender::{TcpMessage, TcpSender};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{split, AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::wrappers::LinesStream;

pub type CoordElection = Arc<Addr<CoordinatorElection>>;

/// This actor is responsible for handling the admin connections and business logic.
/// Handles clients and their requests as well as the communication with the coordinator.
pub struct Admin {
    pub addr: SocketAddr,
    pub client_addr: SocketAddr,
    pub tcp_sender: Arc<Addr<TcpSender>>,
    pub coordinator_election: CoordElection,
    pub coordinator: Arc<Addr<Coordinator>>,
    pub storage_addr: Arc<Addr<Storage>>,
}

impl Actor for Admin {
    type Context = Context<Self>;
}

impl Admin {
    pub fn new(
        stream: TcpStream,
        client_addr: SocketAddr,
        addr: SocketAddr,
        coordinator_election: CoordElection,
        coordinator: Arc<Addr<Coordinator>>,
        storage_addr: Arc<Addr<Storage>>,
    ) -> Addr<Self> {
        Admin::create(|ctx| {
            let (r_half, w_half) = split(stream);
            Admin::add_stream(LinesStream::new(BufReader::new(r_half).lines()), ctx);
            let write = Some(w_half);
            let sender_actor = TcpSender { write }.start();
            let tcp_sender = Arc::new(sender_actor);

            Admin {
                addr,
                client_addr,
                tcp_sender,
                coordinator_election,
                coordinator,
                storage_addr,
            }
        })
    }

    pub async fn start(addr: SocketAddr, peers: Vec<SocketAddr>) -> Result<(), AdminError> {
        println!("[{}] Starting admin server", addr);
        if peers.is_empty() {
            return Err(AdminError::InvalidPeers("No peers provided".to_string()));
        }

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| AdminError::BindError(format!("Failed to bind to {}: {:?}", addr, e)))?;

        let coordinator = Arc::new(Coordinator::new(addr, peers.clone()));

        let coordinator_election = Arc::new(CoordinatorElection::new(
            addr,
            Arc::clone(&coordinator),
            Arc::new(peers),
        ));

        let storage_actor = Arc::new(Storage::start()?);

        spawn_ping_task(coordinator_election.clone());

        spawn_reaper_task(storage_actor.clone(), coordinator.clone());

        accept_connections(
            listener,
            addr,
            coordinator_election,
            coordinator,
            storage_actor,
        )
        .await
    }
}

impl StreamHandler<Result<String, std::io::Error>> for Admin {
    fn handle(&mut self, read: Result<String, std::io::Error>, ctx: &mut Self::Context) {
        if let Ok(line) = read {
            let message = line.trim();

            // PING MESSAGE
            if let Ok(ping_msg) = serde_json::from_str::<PingMessage>(message) {
                log_elections(format!("[{:?}] Received Ping sending Ack", self.addr));
                ctx.address()
                    .try_send(ping_msg)
                    .expect("Failed to send PingMessage");
            }

            // WHO IS COORDINATOR
            if message == "WhoIsCoordinator" {
                ctx.address()
                    .try_send(WhoIsCoordinator {})
                    .expect("Failed to send WhoIsCoordinator");
            }

            // ELECTION MESSAGE
            if let Ok(election_msg) = serde_json::from_str::<ElectionMessage>(message) {
                let tcp_sender_clone = self.tcp_sender.clone();

                tcp_sender_clone
                    .try_send(TcpMessage("Ack".to_string()))
                    .expect("Failed to send response");

                self.coordinator_election
                    .try_send(ElectionMessage {
                        candidates: election_msg.candidates,
                    })
                    .expect("Failed to send election message");
            }

            // COORDINATOR MESSAGE
            if let Ok(coord_msg) = serde_json::from_str::<CoordinatorMessage>(message) {
                let tcp_sender_clone = self.tcp_sender.clone();
                tcp_sender_clone
                    .try_send(TcpMessage("Ack".to_string()))
                    .expect("Failed to send response");
                self.coordinator_election
                    .try_send(CoordinatorMessage {
                        coordinator: coord_msg.coordinator,
                    })
                    .expect("Failed to send coordinator message");
            }

            // TRIP REQUEST
            if let Ok(request_trip) = serde_json::from_str::<RequestTrip>(message) {
                log_trips(format!("[Request trip: {:?}]", self.client_addr));
                ctx.address()
                    .try_send(request_trip)
                    .expect("RequestTrip failed");
            }

            // DRIVER READY
            if let Ok(driver_ready) = serde_json::from_str::<DriverPosition>(message) {
                log_trips(format!("[Driver ready: {:?}]", self.client_addr));
                ctx.address()
                    .try_send(driver_ready)
                    .expect("DriverPosition failed");
            }

            // FINISH TRIP
            if let Ok(finish_trip) = serde_json::from_str::<FinishTrip>(message) {
                ctx.address()
                    .try_send(finish_trip)
                    .expect("FinishTrip failed");
            }

            // HANDLE TRIP
            if let Ok(handle_trip) = serde_json::from_str::<HandleTrip>(message) {
                ctx.address()
                    .try_send(handle_trip)
                    .expect("HandleTrip failed");
            }

            // CAN ACCEPT TRIP RESPONSE
            if let Ok(can_accept_trip_response) =
                serde_json::from_str::<CanAcceptTripResponse>(message)
            {
                ctx.address()
                    .try_send(can_accept_trip_response)
                    .expect("CanAcceptTripResponse failed");
            }

            // MAKE TRIP
            if let Ok(make_trip) = serde_json::from_str::<MakeTrip>(message) {
                self.tcp_sender
                    .try_send(TcpMessage("Ack".to_string()))
                    .expect("Failed to send response");
                ctx.address().try_send(make_trip).expect("MakeTrip failed");
            }

            // INTERNAL STATE UPDATES FROM COORDINATOR
            if let Ok(passenger_update) = serde_json::from_str::<UpdatePassengers>(message) {
                ctx.address()
                    .try_send(MakeUpdatePassenger {
                        upt_msg: passenger_update,
                    })
                    .expect("MakeUpdatePassenger failed to send");
            }

            if let Ok(driver_update) = serde_json::from_str::<UpdateDrivers>(message) {
                ctx.address()
                    .try_send(MakeUpdateDriver {
                        upt_msg: driver_update,
                    })
                    .expect("MakeUpdateDriver failed to send");
            }
        } else {
            println!("[{:?}] Failed to read line {:?}", self.addr, read);
        }
    }
}

async fn accept_connections(
    listener: TcpListener,
    addr: SocketAddr,
    coordinator_election: Arc<Addr<CoordinatorElection>>,
    coordinator: Arc<Addr<Coordinator>>,
    storage_actor: Arc<Addr<Storage>>,
) -> Result<(), AdminError> {
    loop {
        match listener.accept().await {
            Ok((stream, client_addr)) => {
                log_elections(format!(
                    "[{}] Connection received from {:?}",
                    addr, client_addr
                ));
                Admin::new(
                    stream,
                    client_addr,
                    addr,
                    coordinator_election.clone(),
                    Arc::clone(&coordinator),
                    storage_actor.clone(),
                );
            }
            Err(e) => {
                println!("[{}] Failed to accept connection: {:?}", addr, e);
            }
        }
    }
}
