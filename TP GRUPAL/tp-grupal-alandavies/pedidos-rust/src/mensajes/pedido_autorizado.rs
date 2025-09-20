use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct PedidoAutorizado {
    id_restaurante: u32,
    id_pedido: u32,
    monto: f32,
}

impl fmt::Display for PedidoAutorizado {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PedidoAutorizado(restaurante: {}, id: {}, monto: {})",
            self.id_restaurante, self.id_pedido, self.monto
        )
    }
}

impl PedidoAutorizado {
    pub fn new(id_restaurante: u32, id_pedido: u32, monto: f32) -> Self {
        Self {
            id_restaurante,
            id_pedido,
            monto,
        }
    }

    pub fn id_restaurante(&self) -> u32 {
        self.id_restaurante
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }

    pub fn monto(&self) -> f32 {
        self.monto
    }
}
