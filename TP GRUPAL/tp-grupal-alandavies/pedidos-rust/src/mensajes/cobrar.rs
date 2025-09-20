use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct Cobrar {
    pub id_pedido: u32,
}

impl fmt::Display for Cobrar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cobrar(id: {})", self.id_pedido)
    }
}

impl Cobrar {
    pub fn new(id_pedido: u32) -> Self {
        Self { id_pedido }
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }
}
