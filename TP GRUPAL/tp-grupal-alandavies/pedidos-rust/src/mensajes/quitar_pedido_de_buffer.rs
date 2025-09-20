use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
pub struct QuitarPedidoDeBuffer {
    id_pedido: u32,
}

impl fmt::Display for QuitarPedidoDeBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "QuitarPedidoDeBuffer(id: {})", self.id_pedido)
    }
}

impl QuitarPedidoDeBuffer {
    pub fn new(id_pedido: u32) -> Self {
        Self { id_pedido }
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }
}
