use super::storage::Storage;
use super::storage_messages::{
    GetDriver, GetNearestDriver, GetPassenger, InsertDriver, InsertPassenger, IsFinished,
    ReapDeadDrivers, RemoveDriver, RemovePassenger, UpdateDriver,
};
use crate::admin_actor::clients_to_admin::DriverStatus;
use crate::storage_actor::storage_messages::DeadDriver;
use crate::utils::entities::{DriverEntity, PassengerEntity};
use actix::{Context, Handler};
use common::messages::FinishTrip;
use std::net::SocketAddr;

impl Handler<InsertDriver> for Storage {
    type Result = ();

    fn handle(&mut self, msg: InsertDriver, _: &mut Self::Context) {
        println!("[STORAGE] Inserting driver with id {:?}", msg.id);
        match self.drivers.get_mut(&msg.id) {
            Some(driver) => {
                driver.driver_position = msg.driver_position;
                driver.status = msg.status;
                driver.time_stamp = msg.time_stamp;
            }
            None => {
                self.drivers.insert(
                    msg.id,
                    DriverEntity {
                        driver_position: msg.driver_position,
                        current_passenger_id: msg.current_passenger_id,
                        driver_sender: msg.driver_sender,
                        status: msg.status,
                        time_stamp: msg.time_stamp,
                    },
                );
            }
        }
    }
}

impl Handler<InsertPassenger> for Storage {
    type Result = ();

    fn handle(&mut self, msg: InsertPassenger, _: &mut Self::Context) {
        println!("[STORAGE] Inserting passenger with id {:?}", msg.id);
        self.passengers.insert(
            msg.id,
            PassengerEntity {
                passenger_position: msg.passenger_position,
                passenger_destination: msg.passenger_destination,
                passenger_sender: msg.passenger_sender,
            },
        );
    }
}

impl Handler<GetPassenger> for Storage {
    type Result = Option<PassengerEntity>;

    fn handle(&mut self, msg: GetPassenger, _: &mut Self::Context) -> Self::Result {
        println!("[STORAGE] Getting passenger with id {:?}", msg.id);
        self.passengers.get(&msg.id).cloned()
    }
}

impl Handler<GetDriver> for Storage {
    type Result = Option<DriverEntity>;

    fn handle(&mut self, msg: GetDriver, _: &mut Self::Context) -> Self::Result {
        println!("[STORAGE - GET] Getting driver with id {:?}", msg.id);
        let driver = self.drivers.get(&msg.id).cloned();
        if driver.is_none() {
            eprintln!("[STORAGE] Driver with id {:?} not found", msg.id);
        }
        driver
    }
}

impl Handler<UpdateDriver> for Storage {
    type Result = ();

    fn handle(&mut self, msg: UpdateDriver, _: &mut Self::Context) {
        println!(
            "[STORAGE- UPDATE] Updating driver with id {:?}",
            msg.driver_id
        );

        if let Some(driver) = self.drivers.get_mut(&msg.driver_id) {
            driver.status = msg.status;
            driver.current_passenger_id = msg.passenger_id;
            driver.time_stamp = msg.time_stamp;
        } else {
            eprintln!("Driver with id {:?} not found", msg.driver_id);
        }
    }
}

impl Handler<RemoveDriver> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemoveDriver, _: &mut Self::Context) {
        println!("[STORAGE - DELETE] Removing driver with id {:?}", msg.id);
        self.drivers.remove(&msg.id);
    }
}

impl Handler<RemovePassenger> for Storage {
    type Result = ();

    fn handle(&mut self, msg: RemovePassenger, _: &mut Self::Context) {
        println!("[STORAGE - DELETE] Removing passenger with id {:?}", msg.id);
        self.passengers.remove(&msg.id);
    }
}

impl Handler<GetNearestDriver> for Storage {
    type Result = Option<SocketAddr>;

    fn handle(&mut self, _: GetNearestDriver, _: &mut Self::Context) -> Self::Result {
        println!("[STORAGE - GET] Getting nearest driver");
        let mut nearest_driver_addr: Option<SocketAddr> = None;
        let mut nearest_distance = f32::INFINITY;

        for (addr, driver) in self.drivers.iter() {
            if driver.status == DriverStatus::Active {
                let distance = ((driver.driver_position.0).powi(2)
                    + (driver.driver_position.1).powi(2))
                .sqrt();
                if distance < nearest_distance {
                    nearest_distance = distance;
                    nearest_driver_addr = Some(*addr);
                }
            }
        }
        println!(
            "[STORAGE - GET] Nearest driver is {:?}",
            nearest_driver_addr
        );

        nearest_driver_addr
    }
}

impl Handler<FinishTrip> for Storage {
    type Result = ();

    fn handle(&mut self, msg: FinishTrip, _: &mut Self::Context) {
        println!(
            "[STORAGE - UPDATE] Finishing trip for passenger with id {:?}",
            msg.passenger_id_ft
        );
        if let Some(driver) = self.drivers.get_mut(&msg.driver_id_ft) {
            driver.status = DriverStatus::Active;
            driver.time_stamp = std::time::Instant::now();
            driver.driver_position = msg.destination_pos;
        }
        if self.passengers.remove(&msg.passenger_id_ft).is_none() {
            eprintln!(
                "[STORAGE] Passenger with id {:?} not found",
                msg.passenger_id_ft
            );
        }
    }
}

impl Handler<IsFinished> for Storage {
    type Result = bool;

    fn handle(&mut self, msg: IsFinished, _: &mut Self::Context) -> Self::Result {
        println!(
            "[STORAGE] Checking if passenger with id {:?} is finished",
            msg.passenger_id
        );
        !self.passengers.contains_key(&msg.passenger_id)
    }
}

impl Handler<ReapDeadDrivers> for Storage {
    type Result = Vec<DeadDriver>;

    fn handle(&mut self, _: ReapDeadDrivers, _: &mut Context<Self>) -> Vec<DeadDriver> {
        println!("[STORAGE - REAPER] Reaping dead drivers . . .");
        let mut dead_drivers = Vec::new();
        for (driver_id, driver) in self.drivers.iter_mut() {
            if matches!(driver.status, DriverStatus::Waiting)
                && driver.time_stamp.elapsed().as_secs() > 3
            {
                let passenger_id = driver.current_passenger_id;
                let passenger_sender = self
                    .passengers
                    .get(&passenger_id.unwrap())
                    .unwrap()
                    .passenger_sender
                    .clone();

                let dead_driver = DeadDriver {
                    driver_id: *driver_id,
                    passenger_id,
                    passenger_sender,
                };

                dead_drivers.push(dead_driver);
            }
        }
        if !dead_drivers.is_empty() {
            println!(
                "[STORAGE - REAPER] Reaped {} dead drivers",
                dead_drivers.len()
            );
        }

        for dead_driver in &dead_drivers {
            self.drivers.remove(&dead_driver.driver_id);
            self.passengers.remove(&dead_driver.passenger_id.unwrap());
        }

        dead_drivers
    }
}
