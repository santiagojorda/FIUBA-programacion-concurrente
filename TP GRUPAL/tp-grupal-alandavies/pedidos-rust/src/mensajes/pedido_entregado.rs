use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct PedidoEntregado {
    id_pedido: u32,
    id_repartidor: u32,
}

impl PedidoEntregado {
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

impl fmt::Display for PedidoEntregado {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PedidoEntregado(repartidor: {}, id: {})",
            self.id_repartidor, self.id_pedido
        )
    }
}
