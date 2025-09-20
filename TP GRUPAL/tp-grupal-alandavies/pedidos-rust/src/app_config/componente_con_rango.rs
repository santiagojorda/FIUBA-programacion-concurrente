use std::net::Ipv4Addr;

use serde::Deserialize;

use crate::app_config::deserialize_ip::deserialize_ip;

#[derive(Debug, Deserialize)]
pub struct ComponenteConRango {
    #[serde(deserialize_with = "deserialize_ip")]
    ip: Ipv4Addr,
    rango_puertos: (u16, u16),
}

impl ComponenteConRango {
    pub fn ip(&self) -> Ipv4Addr {
        self.ip
    }

    pub fn rango_puertos(&self) -> (u16, u16) {
        self.rango_puertos
    }

    pub fn validar_rango_puertos(&self) -> bool {
        let (inicio, fin) = self.rango_puertos;
        inicio < fin
    }
}
