use crate::admin_actor::admin::Admin;
use crate::admin_actor::clients_to_admin::DriverStatus;
use crate::coordinator_actor::coordinator_messages::{Action, UpdateDrivers, UpdatePassengers};
use crate::storage_actor::storage_messages::{
    InsertDriver, InsertPassenger, RemoveDriver, RemovePassenger, UpdateDriver,
};
use crate::utils::logs::log_trips;
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
/// Message to update the driver in the storage.
pub struct MakeUpdateDriver {
    pub upt_msg: UpdateDrivers,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
/// Message to update the passenger in the storage.
pub struct MakeUpdatePassenger {
    pub upt_msg: UpdatePassengers,
}

impl Handler<MakeUpdateDriver> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(
        &mut self,
        driver_update: MakeUpdateDriver,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let d_addr = driver_update.upt_msg.driver;
        let position = driver_update.upt_msg.position;
        let action = driver_update.upt_msg.action;
        let passenger = driver_update.upt_msg.current_passenger_id;
        let driver_status = driver_update.upt_msg.status;
        let tcp_sender_clone = self.tcp_sender.clone();
        let storage_actor = self.storage_addr.clone();

        Box::pin(
            async move {
                println!(
                    "[ Driver Update - Address: {:?}, Position: {:?}, Action: {:?}, Passenger: {:?}, Status: {:?} ]",
                    d_addr, position, action, passenger, driver_status
                );
                if action == Action::Insert {
                    storage_actor
                        .send(InsertDriver {
                            id: d_addr,
                            driver_position: position,
                            current_passenger_id: None,
                            driver_sender: Some(tcp_sender_clone),
                            status: DriverStatus::Active,
                            time_stamp: std::time::Instant::now(),
                        })
                        .await
                        .expect("Failed to send AddDriver to storage");

                    log_trips(format!("Added driver {:?}", d_addr));
                } else if action == Action::Delete {
                    storage_actor
                        .send(RemoveDriver { id: d_addr })
                        .await
                        .expect("Failed to send RemoveDriver to storage");

                    log_trips(format!("Removed driver {:?}", d_addr));
                } else if action == Action::Update {
                    storage_actor
                        .send(UpdateDriver {
                            driver_id: d_addr,
                            passenger_id: passenger,
                            status: driver_status,
                            time_stamp: Instant::now(),
                        })
                        .await
                        .expect("Failed to send RemoveDriver to storage");

                    log_trips(format!("Updated driver with id: {:?}", d_addr));
                }
            }.into_actor(self),
        )
    }
}

impl Handler<MakeUpdatePassenger> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(
        &mut self,
        passenger_update: MakeUpdatePassenger,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let p_addr = passenger_update.upt_msg.passenger;
        let action = passenger_update.upt_msg.action;
        let storage_actor = self.storage_addr.clone();
        Box::pin(
            async move {
                if action == Action::Insert {
                    storage_actor
                        .send(InsertPassenger {
                            id: p_addr,
                            passenger_position: passenger_update.upt_msg.origin,
                            passenger_destination: passenger_update.upt_msg.destination,
                            passenger_sender: None,
                        })
                        .await
                        .expect("Failed to send AddPassenger to storage");

                    log_trips(format!("Added passenger {:?}", p_addr));
                } else if action == Action::Delete {
                    storage_actor
                        .send(RemovePassenger { id: p_addr })
                        .await
                        .expect("Failed to send GetPassenger to storage");

                    log_trips(format!("Removed passenger {:?}", p_addr));
                }
            }
            .into_actor(self),
        )
    }
}
