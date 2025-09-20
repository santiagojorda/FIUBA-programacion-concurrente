use crate::estructuras_aux::ubicacion::Ubicacion;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct SolicitarPedidos {
    id: u32,
    ubicacion: Ubicacion,
}

impl fmt::Display for SolicitarPedidos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SolicitarPedidos(id: {}, ubicacion: {:?})",
            self.id, self.ubicacion
        )
    }
}

impl SolicitarPedidos {
    pub fn new(id: u32, ubicacion: Ubicacion) -> Self {
        Self { id, ubicacion }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn ubicacion(&self) -> &Ubicacion {
        &self.ubicacion
    }
}
