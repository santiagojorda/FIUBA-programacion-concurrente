use crate::app_config::deserialize_ip::deserialize_ip;
use serde::Deserialize;
use std::net::Ipv4Addr;

#[derive(Debug, Deserialize)]
pub(crate) struct Componente {
    #[serde(deserialize_with = "deserialize_ip")]
    ip: Ipv4Addr,
}

impl Componente {
    pub fn ip(&self) -> Ipv4Addr {
        self.ip
    }
}
