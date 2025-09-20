use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct PedidoAsignadoAOtro {
    id_pedido: u32,
}

impl fmt::Display for PedidoAsignadoAOtro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PedidoAsignadoAOtro(id: {})", self.id_pedido)
    }
}

impl PedidoAsignadoAOtro {
    pub fn new(id_pedido: u32) -> Self {
        Self { id_pedido }
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }
}
