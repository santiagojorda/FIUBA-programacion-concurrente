use crate::app_config::deserialize_ip::deserialize_ip;
use serde::Deserialize;
use std::net::Ipv4Addr;

#[derive(Debug, Deserialize)]
pub(crate) struct ComponenteConPuerto {
    #[serde(deserialize_with = "deserialize_ip")]
    ip: Ipv4Addr,
    puerto: u16,
}

impl ComponenteConPuerto {
    pub fn ip(&self) -> Ipv4Addr {
        self.ip
    }

    pub fn puerto(&self) -> u16 {
        self.puerto
    }
}
