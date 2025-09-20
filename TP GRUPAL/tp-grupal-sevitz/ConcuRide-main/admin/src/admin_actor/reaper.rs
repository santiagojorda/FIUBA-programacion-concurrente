use crate::admin_actor::clients_to_admin::DriverStatus;
use crate::coordinator_actor::coordinator::Coordinator;
use crate::coordinator_actor::coordinator_messages::{Action, UpdateDrivers, UpdatePassengers};
use crate::storage_actor::storage::Storage;
use crate::storage_actor::storage_messages::ReapDeadDrivers;
use actix::Addr;
use common::messages::RejectTrip;
use common::tcp_sender::TcpMessage;
use std::sync::Arc;
use std::time::Duration;

/// Function to remove dead drivers from the system.
/// (Driver who never responded if they can accept a trip)
async fn reap_dead_drivers(storage_actor: Arc<Addr<Storage>>, coord_addr: Arc<Addr<Coordinator>>) {
    let reaped_drivers = storage_actor.send(ReapDeadDrivers).await.unwrap();

    if !reaped_drivers.is_empty() {
        println!("Reaped {} dead drivers", reaped_drivers.len());

        for dead_driver in &reaped_drivers {
            coord_addr
                .try_send(UpdateDrivers {
                    action: Action::Delete,
                    driver: dead_driver.driver_id,
                    position: (0.0, 0.0),
                    current_passenger_id: None,
                    status: DriverStatus::Active,
                })
                .expect("Failed to send UpdateDrivers");

            coord_addr
                .try_send(UpdatePassengers {
                    action: Action::Delete,
                    passenger: dead_driver
                        .passenger_id
                        .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap()),
                    origin: (0.0, 0.0),
                    destination: (0.0, 0.0),
                })
                .expect("Failed to send UpdateDrivers");

            let tcp_message = TcpMessage(
                serde_json::to_string(&RejectTrip {
                    response: "your driver was disconnected, please try again".to_string(),
                })
                .unwrap(),
            );
            if let Some(sender) = dead_driver.passenger_sender.as_ref() {
                sender
                    .send(tcp_message)
                    .await
                    .expect("Failed to send RejectTrip");
            }

            println!("Reaped driver with id {:?}", dead_driver.driver_id);
        }
    }
}

pub fn spawn_reaper_task(storage_actor: Arc<Addr<Storage>>, coord_addr: Arc<Addr<Coordinator>>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            reap_dead_drivers(storage_actor.clone(), coord_addr.clone()).await;
        }
    });
}
