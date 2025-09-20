use crate::estructuras_aux::ubicacion::Ubicacion;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
pub struct PedidoListo {
    id_pedido: u32,
    id_restaurante: u32,
    ubicacion_comensal: Ubicacion,
    ubicacion_restaurante: Ubicacion,
    direccion_comensal: SocketAddr,
}

impl fmt::Display for PedidoListo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PedidoListo(id: {}, ubicacion_comensal: {:?}, ubicacion_restaurante: {:?})",
            self.id_pedido, self.ubicacion_comensal, self.ubicacion_restaurante
        )
    }
}

impl PedidoListo {
    pub fn new(
        id_pedido: u32,
        id_restaurante: u32,
        ubicacion_comensal: Ubicacion,
        ubicacion_restaurante: Ubicacion,
        direccion_comensal: SocketAddr,
    ) -> Self {
        Self {
            id_pedido,
            id_restaurante,
            ubicacion_comensal,
            ubicacion_restaurante,
            direccion_comensal,
        }
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }

    pub fn id_restaurante(&self) -> u32 {
        self.id_restaurante
    }

    pub fn ubicacion_comensal(&self) -> &Ubicacion {
        &self.ubicacion_comensal
    }

    pub fn ubicacion_restaurante(&self) -> &Ubicacion {
        &self.ubicacion_restaurante
    }

    pub fn direccion_comensal(&self) -> &SocketAddr {
        &self.direccion_comensal
    }
}
