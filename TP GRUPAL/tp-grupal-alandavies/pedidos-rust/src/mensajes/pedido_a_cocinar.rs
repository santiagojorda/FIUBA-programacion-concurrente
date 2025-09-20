use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct PedidoACocinar {
    id_pedido: u32,
}

impl PedidoACocinar {
    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }

    pub fn new(id_pedido: u32) -> Self {
        Self { id_pedido }
    }
}
