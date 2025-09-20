use crate::admin_actor::admin::Admin;
use crate::coordinator_actor::coordinator_messages::{Action, UpdateDrivers, UpdatePassengers};
use crate::elections::election_messages::AmICoordinator;
use crate::storage_actor::storage_messages::{InsertDriver, InsertPassenger};
use crate::utils::logs::log_trips;
use crate::utils::payment_actions::{get_payment_response, make_payment_check_message};
use actix::prelude::*;
use common::messages::{AuthConfirmation, DriverPosition, RequestTrip};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize, Message, PartialEq)]
#[rtype(result = "()")]
/// Enum to represent the status of a driver.
/// Active: The driver is available to accept a trip.
/// Waiting: The driver hasn't responded if can accept trip yet.
/// OnTrip: The driver is currently on a trip.
pub enum DriverStatus {
    Active,
    Waiting,
    OnTrip,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
/// Message to inform the Admin that a trip has finished.
pub struct TripFinished {
    pub passenger_id_tf: SocketAddr,
    pub driver_id_tf: SocketAddr,
}

impl Handler<RequestTrip> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: RequestTrip, ctx: &mut Self::Context) -> Self::Result {
        log_trips("| ----- Request Trip ----- | ".to_string());

        let tcp_sender = self.tcp_sender.clone();
        let client_addr = self.client_addr;
        let storage_actor = self.storage_addr.clone();

        let cord_election_clone = self.coordinator_election.clone();
        let cord_clone = self.coordinator.clone();
        let adress = ctx.address();

        Box::pin(
            async move {
                if cord_election_clone
                    .send(AmICoordinator)
                    .await
                    .unwrap_or(false)
                {
                    storage_actor
                        .send(InsertPassenger {
                            id: client_addr,
                            passenger_position: msg.origin,
                            passenger_destination: msg.destination,
                            passenger_sender: Some(tcp_sender),
                        })
                        .await
                        .unwrap();

                    if let Err(e) = cord_clone
                        .send(UpdatePassengers {
                            passenger: client_addr,
                            origin: msg.origin,
                            destination: msg.destination,
                            action: Action::Insert,
                        })
                        .await
                    {
                        log_trips(format!("Failed to update passengers: {:?}", e));
                    }

                    // check payment request
                    let passenger_id = format!("{:?}", client_addr.clone());
                    let auth = get_payment_response(make_payment_check_message(passenger_id)).await;
                    log_trips(format!("Payment response is authorized: {:?}", auth));
                    adress
                        .try_send(AuthConfirmation {
                            passenger_id_ac: client_addr,
                            is_authorized: auth,
                        })
                        .expect("failed to send auth confirmation");
                }
            }
            .into_actor(self),
        )
    }
}

impl Handler<DriverPosition> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: DriverPosition, _ctx: &mut Self::Context) -> Self::Result {
        log_trips("| ----- Driver Position Ready ----- |".to_string());

        let cord_election_clone = self.coordinator_election.clone();
        let cord_clone = self.coordinator.clone();

        let tcp_sender = self.tcp_sender.clone();
        let client_addr = self.client_addr;
        let storage_actor = self.storage_addr.clone();

        Box::pin(
            async move {
                if cord_election_clone
                    .send(AmICoordinator)
                    .await
                    .unwrap_or(false)
                {
                    storage_actor
                        .send(InsertDriver {
                            id: client_addr,
                            current_passenger_id: None,
                            driver_position: msg.position,
                            driver_sender: Some(tcp_sender),
                            status: DriverStatus::Active,
                            time_stamp: std::time::Instant::now(),
                        })
                        .await
                        .unwrap();

                    if let Err(e) = cord_clone
                        .send(UpdateDrivers {
                            driver: client_addr,
                            position: msg.position,
                            action: Action::Insert,
                            current_passenger_id: None,
                            status: DriverStatus::Active,
                        })
                        .await
                    {
                        log_trips(format!("Failed to update drivers: {:?}", e));
                    }
                }
            }
            .into_actor(self),
        )
    }
}
