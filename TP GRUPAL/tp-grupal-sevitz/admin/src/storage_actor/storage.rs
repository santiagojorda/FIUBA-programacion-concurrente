use crate::utils::{
    admin_errors::AdminError,
    entities::{DriverEntity, PassengerEntity},
};
use actix::Addr;
use actix::{Actor, Context};
use std::{collections::HashMap, net::SocketAddr};

/// This actor is responsible for storing the passengers and drivers in the system.
pub struct Storage {
    pub passengers: HashMap<SocketAddr, PassengerEntity>,
    pub drivers: HashMap<SocketAddr, DriverEntity>,
}

impl Actor for Storage {
    type Context = Context<Self>;
}

impl Storage {
    pub fn start() -> Result<Addr<Storage>, AdminError> {
        println!("[STORAGE] Starting storage actor");
        let storage = Storage {
            passengers: HashMap::new(),
            drivers: HashMap::new(),
        };

        let storage_addr = storage.start();

        Ok(storage_addr)
    }
}
