use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddr};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct LlegoPedido {
    id_pedido: u32,
    direccion_comensal: SocketAddr,
}

impl fmt::Display for LlegoPedido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LlegoPedido(id_pedido: {}, direccion_comensal: {})",
            self.id_pedido, self.direccion_comensal
        )
    }
}

impl LlegoPedido {
    pub fn new(id_pedido: u32, direccion_comensal: SocketAddr) -> Self {
        Self {
            id_pedido,
            direccion_comensal,
        }
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }

    pub fn direccion_comensal(&self) -> &SocketAddr {
        &self.direccion_comensal
    }
}
