use crate::admin_actor::{admin::Admin, clients_to_admin::DriverStatus};
use crate::coordinator_actor::coordinator_messages::{
    Action, HandleTrip, UpdateDrivers, UpdatePassengers,
};
use crate::storage_actor::storage::Storage;
use crate::storage_actor::storage_messages::{GetDriver, GetPassenger, IsFinished, UpdateDriver};
use crate::utils::logs::log_trips;
use crate::utils::payment_actions::{get_payment_response, make_payment_done_message};
use actix::prelude::*;
use common::{
    messages::{CanAcceptTripResponse, FinishTrip, StartTrip},
    tcp_sender::TcpMessage,
};
use std::sync::Arc;
use std::time::Instant;

impl Handler<FinishTrip> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: FinishTrip, _ctx: &mut Self::Context) -> Self::Result {
        log_trips(format!(
            "| ----- Finish Trip {:?} ----- |",
            self.client_addr
        ));

        let storage_actor = self.storage_addr.clone();
        let driver_id = msg.driver_id_ft;
        let passenger_id = msg.passenger_id_ft;
        let destination_pos = msg.destination_pos;

        let coord_clone = self.coordinator.clone();

        Box::pin(
            async move {
                // if the passenger has already finished the trip, return
                if storage_actor
                    .send(IsFinished { passenger_id })
                    .await
                    .unwrap()
                {
                    return;
                }

                let string_passenger_id = format!("{:?}", passenger_id.clone());
                get_payment_response(make_payment_done_message(string_passenger_id)).await;

                acknowledge_passenger(storage_actor.clone(), msg.clone()).await;
                acknowledge_driver(storage_actor.clone(), msg).await;

                storage_actor
                    .send(FinishTrip {
                        passenger_id_ft: passenger_id,
                        driver_id_ft: driver_id,
                        destination_pos,
                    })
                    .await
                    .unwrap();

                coord_clone
                    .try_send(UpdateDrivers {
                        driver: driver_id,
                        position: (destination_pos.0, destination_pos.1),
                        action: Action::Update,
                        current_passenger_id: None,
                        status: DriverStatus::Active,
                    })
                    .expect("Failed to send UpdateDrivers");

                coord_clone
                    .try_send(UpdatePassengers {
                        passenger: passenger_id,
                        origin: (0.0, 0.0),
                        destination: (20.0, 20.0),
                        action: Action::Delete,
                    })
                    .expect("Failed to send UpdatePassengers");
            }
            .into_actor(self),
        )
    }
}

async fn acknowledge_passenger(storage: Arc<Addr<Storage>>, msg: FinishTrip) {
    let passenger = storage
        .send(GetPassenger {
            id: msg.passenger_id_ft,
        })
        .await
        .unwrap();
    if let Some(passenger_entity) = passenger {
        if let Some(passenger_sender) = passenger_entity.passenger_sender.as_ref() {
            passenger_sender
                .try_send(TcpMessage("ACK".to_string()))
                .expect("Failed to send FinishTrip");
        }
    }
}

async fn acknowledge_driver(storage: Arc<Addr<Storage>>, msg: FinishTrip) {
    let driver = storage
        .send(GetDriver {
            id: msg.driver_id_ft,
        })
        .await
        .unwrap();
    if let Some(driver_entity) = driver {
        if let Some(passenger_sender) = driver_entity.driver_sender.as_ref() {
            passenger_sender
                .try_send(TcpMessage("ACK".to_string()))
                .expect("Failed to send FinishTrip");
        }
    }
}

impl Handler<CanAcceptTripResponse> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: CanAcceptTripResponse, _ctx: &mut Self::Context) -> Self::Result {
        log_trips("| ----- Can Accept Trip Response ----- |".to_string());

        let coord_clone = self.coordinator.clone();
        let storage_actor = self.storage_addr.clone();
        let driver_sender = self.tcp_sender.clone();
        let driver_id = self.client_addr;

        Box::pin(
            async move {
                if msg.is_accepted {
                    // Start the trip
                    log_trips(format!(
                        "| ----- Start Trip {:?} ----- |",
                        msg.passenger_id_car
                    ));

                    let passenger_entity_opt = storage_actor
                        .send(GetPassenger {
                            id: msg.passenger_id_car,
                        })
                        .await
                        .unwrap();

                    if let Some(passenger_entity) = passenger_entity_opt {
                        let start_msg = StartTrip {
                            passenger_id_st: msg.passenger_id_car,
                            driver_id_st: driver_id,
                            origin: passenger_entity.passenger_position,
                            destination: passenger_entity.passenger_destination,
                        };
                        let passenger_sender = passenger_entity.passenger_sender.as_ref();
                        if let Some(passenger_sender) = passenger_sender {
                            match serde_json::to_string(&start_msg) {
                                Ok(json_string) => {
                                    passenger_sender
                                        .try_send(TcpMessage(json_string.clone()))
                                        .expect("Failed to send StartTrip");
                                    driver_sender
                                        .try_send(TcpMessage(json_string))
                                        .expect("Failed to send StartTrip");
                                }
                                Err(err) => {
                                    eprintln!("Error serializing StartTrip message: {}", err);
                                }
                            }

                            storage_actor
                                .send(UpdateDriver {
                                    driver_id,
                                    passenger_id: Some(msg.passenger_id_car),
                                    time_stamp: Instant::now(),
                                    status: DriverStatus::OnTrip,
                                })
                                .await
                                .unwrap();

                            coord_clone
                                .try_send(UpdateDrivers {
                                    driver: driver_id,
                                    position: (0.0, 0.1),
                                    action: Action::Update,
                                    current_passenger_id: Some(msg.passenger_id_car),
                                    status: DriverStatus::OnTrip,
                                })
                                .expect("Failed to send UpdateDrivers");
                        } else {
                            println!(
                                "Passenger sender not found for passenger {:?}",
                                msg.passenger_id_car
                            );
                        }
                    }
                } else {
                    storage_actor
                        .send(UpdateDriver {
                            driver_id,
                            passenger_id: Some(msg.passenger_id_car),
                            time_stamp: Instant::now(),
                            status: DriverStatus::Active,
                        })
                        .await
                        .unwrap();

                    coord_clone
                        .try_send(UpdateDrivers {
                            driver: driver_id,
                            position: (0.0, 0.1),
                            action: Action::Update,
                            current_passenger_id: Some(msg.passenger_id_car),
                            status: DriverStatus::Active,
                        })
                        .expect("Failed to send UpdateDrivers");

                    // Find another driver
                    coord_clone
                        .try_send(HandleTrip {
                            passenger_id_ht: msg.passenger_id_car,
                        })
                        .expect("Failed to send HandleTrip");
                }
            }
            .into_actor(self),
        )
    }
}
