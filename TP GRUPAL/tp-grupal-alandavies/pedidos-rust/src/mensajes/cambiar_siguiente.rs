use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct CambiarSiguiente {
    siguiente: SocketAddrV4,
}

impl fmt::Display for CambiarSiguiente {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CambiarSiguiente(siguiente: {})", self.siguiente)
    }
}

impl CambiarSiguiente {
    pub fn new(siguiente: SocketAddrV4) -> Self {
        Self { siguiente }
    }

    pub fn siguiente(&self) -> &SocketAddrV4 {
        &self.siguiente
    }
}
