use crate::estructuras_aux::ubicacion::Ubicacion;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct VerRestaurantes {
    comensal: SocketAddrV4,
    ubicacion_comensal: Ubicacion,
}

impl fmt::Display for VerRestaurantes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VerRestaurantes(ubicacion: {:?})",
            self.ubicacion_comensal
        )
    }
}

impl VerRestaurantes {
    pub fn new(comensal: SocketAddrV4, ubicacion_comensal: Ubicacion) -> Self {
        Self {
            comensal,
            ubicacion_comensal,
        }
    }

    pub fn ubicacion_comensal(&self) -> &Ubicacion {
        &self.ubicacion_comensal
    }

    pub fn address_comensal(&self) -> &SocketAddrV4 {
        &self.comensal
    }
}
