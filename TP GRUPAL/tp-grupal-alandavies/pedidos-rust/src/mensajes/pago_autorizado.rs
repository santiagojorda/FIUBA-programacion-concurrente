use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::estructuras_aux::ubicacion::Ubicacion;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct PagoAutorizado {
    pub id_pedido: u32,
    pub monto: f32,
    pub id_restaurante: u32,
    pub ubicacion_comensal: Ubicacion,
}

impl fmt::Display for PagoAutorizado {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PagoAutorizado(id: {})", self.id_pedido)
    }
}

impl PagoAutorizado {
    pub fn new(
        id_pedido: u32,
        monto: f32,
        id_restaurante: u32,
        ubicacion_comensal: Ubicacion,
    ) -> Self {
        PagoAutorizado {
            id_pedido,
            monto,
            id_restaurante,
            ubicacion_comensal,
        }
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }
}
