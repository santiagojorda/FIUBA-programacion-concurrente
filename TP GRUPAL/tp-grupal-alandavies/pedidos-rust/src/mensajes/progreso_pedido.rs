use crate::estructuras_aux::estado_pedido::EstadoPedido;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
pub struct ProgresoPedido {
    estado_pedido: EstadoPedido,
    id_pedido: u32,
}

impl fmt::Display for ProgresoPedido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProgresoPedido(id: {}, estado: {:?})",
            self.id_pedido, self.estado_pedido
        )
    }
}

impl ProgresoPedido {
    pub fn new(estado_pedido: EstadoPedido, id_pedido: u32) -> Self {
        Self {
            estado_pedido,
            id_pedido,
        }
    }

    pub fn estado_pedido(&self) -> &EstadoPedido {
        &self.estado_pedido
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }
}
