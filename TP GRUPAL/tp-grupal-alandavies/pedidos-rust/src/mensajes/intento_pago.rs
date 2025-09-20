use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};

use crate::estructuras_aux::ubicacion::Ubicacion;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct IntentoPago {
    pub id_pedido: u32,
    pub monto: f32,
    pub comensal_ip: SocketAddrV4,
    pub id_restaurante: u32,
    pub ubicacion_comensal: Ubicacion,
}

impl fmt::Display for IntentoPago {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IntentoPago(id: {}, monto: {})",
            self.id_pedido, self.monto
        )
    }
}

impl IntentoPago {
    pub fn new(
        id_pedido: u32,
        monto: f32,
        comensal_ip: SocketAddrV4,
        id_restaurante: u32,
        ubicacion_comensal: Ubicacion,
    ) -> Self {
        IntentoPago {
            id_pedido,
            monto,
            comensal_ip,
            id_restaurante,
            ubicacion_comensal,
        }
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }

    pub fn monto(&self) -> f32 {
        self.monto
    }
}
