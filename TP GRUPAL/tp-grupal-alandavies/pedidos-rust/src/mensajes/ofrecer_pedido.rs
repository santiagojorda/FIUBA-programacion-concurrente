use crate::mensajes::pedido_listo::PedidoListo;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "bool")]
pub struct OfrecerPedido {
    pedido: PedidoListo,
}

impl fmt::Display for OfrecerPedido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OfrecerPedido(pedido: {:?})", self.pedido)
    }
}

impl OfrecerPedido {
    pub fn new(pedido: PedidoListo) -> Self {
        Self { pedido }
    }

    pub fn pedido(&self) -> &PedidoListo {
        &self.pedido
    }
}
