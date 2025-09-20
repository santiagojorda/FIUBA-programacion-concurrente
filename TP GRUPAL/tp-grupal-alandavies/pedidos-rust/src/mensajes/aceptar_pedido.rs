use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct AceptarPedido {
    id_pedido: u32,
    id_repartidor: u32,
}

impl fmt::Display for AceptarPedido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AceptarPedido(id: {}, repartidor: {})",
            self.id_pedido, self.id_repartidor
        )
    }
}

impl AceptarPedido {
    pub fn new(id_pedido: u32, id_repartidor: u32) -> Self {
        Self {
            id_pedido,
            id_repartidor,
        }
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }

    pub fn id_repartidor(&self) -> u32 {
        self.id_repartidor
    }
}
