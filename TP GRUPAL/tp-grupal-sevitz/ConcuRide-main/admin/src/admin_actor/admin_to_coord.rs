use crate::{
    admin_actor::{admin::Admin, clients_to_admin::DriverStatus},
    coordinator_actor::coordinator_messages::HandleTrip,
    storage_actor::{
        storage::Storage,
        storage_messages::{GetDriver, GetPassenger, UpdateDriver},
    },
    utils::logs::log_trips,
};
use actix::prelude::*;
use common::messages::{CanAcceptTrip, RejectTrip};
use common::tcp_sender::TcpMessage;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
/// Message from non coord to coord to make a trip with the given passenger and driver.
pub struct MakeTrip {
    pub passenger_id_mt: SocketAddr,
    pub driver_id_mt: SocketAddr,
}

impl Handler<MakeTrip> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: MakeTrip, _ctx: &mut Self::Context) -> Self::Result {
        log_trips("| ----- Make Trip ----- |".to_string());

        let passenger = msg.passenger_id_mt;
        let coord_clone = self.coordinator.clone();
        let storage_actor = self.storage_addr.clone();

        Box::pin(
            async move {
                if let Ok(Some(driver)) = storage_actor
                    .send(GetDriver {
                        id: msg.driver_id_mt,
                    })
                    .await
                {
                    if matches!(driver.status, DriverStatus::OnTrip) {
                        log_trips("Driver is already on trip".to_string());
                        coord_clone
                            .send(HandleTrip {
                                passenger_id_ht: passenger,
                            })
                            .await
                            .expect("Failed to send HandleTrip");
                        return;
                    }

                    if matches!(driver.status, DriverStatus::Waiting) {
                        log_trips("Driver is already waiting".to_string());

                        coord_clone
                            .send(HandleTrip {
                                passenger_id_ht: passenger,
                            })
                            .await
                            .expect("Failed to send HandleTrip");
                        return;
                    }

                    if let Some(sender) = &driver.driver_sender {
                        let can_accept_trip = CanAcceptTrip {
                            passenger_id_ca: passenger,
                        };
                        match serde_json::to_string(&can_accept_trip) {
                            Ok(json_string) => {
                                // Send the message to the driver
                                sender
                                    .try_send(TcpMessage(json_string))
                                    .expect("Failed to send request trip to nearest driver");
                            }
                            Err(err) => {
                                eprintln!("Error serializing StartTrip message: {}", err);
                            }
                        }

                        storage_actor
                            .send(UpdateDriver {
                                driver_id: msg.driver_id_mt,
                                status: DriverStatus::Waiting,
                                passenger_id: Some(passenger),
                                time_stamp: std::time::Instant::now(),
                            })
                            .await
                            .expect("Failed to update driver status");
                    } else {
                        Self::reject_passenger(
                            &msg,
                            storage_actor,
                            "Driver sender not available".to_string(),
                        )
                        .await;
                        println!("Driver sender not available");
                    }
                } else {
                    // No driver found send reject trip to passenger
                    Self::reject_passenger(
                        &msg,
                        storage_actor,
                        "No drivers are currently available".to_string(),
                    )
                    .await;
                }
            }
            .into_actor(self),
        )
    }
}

impl Admin {
    async fn reject_passenger(
        msg: &MakeTrip,
        storage_actor: Arc<Addr<Storage>>,
        reject_message: String,
    ) {
        if let Ok(Some(rejected_passenger)) = storage_actor
            .send(GetPassenger {
                id: msg.passenger_id_mt,
            })
            .await
        {
            let tcp_message = TcpMessage(
                serde_json::to_string(&RejectTrip {
                    response: reject_message,
                })
                .unwrap(),
            );
            if let Some(sender) = rejected_passenger.passenger_sender.as_ref() {
                sender
                    .send(tcp_message)
                    .await
                    .expect("Failed to send RejectTrip");
            }
        }
        log_trips("No driver found for the passenger".to_string());
    }
}
