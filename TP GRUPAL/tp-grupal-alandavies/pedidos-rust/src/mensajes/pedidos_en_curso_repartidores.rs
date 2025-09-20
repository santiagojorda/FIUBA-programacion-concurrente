use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::mensajes::info_repartidor_pedido::InfoRepartidorPedido;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct PedidosEnCursoRepartidores {
    pedidos: Vec<InfoRepartidorPedido>,
}

impl fmt::Display for PedidosEnCursoRepartidores {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PedidosEnCursoRepartidores(pedidos: {:?})", self.pedidos)
    }
}

impl PedidosEnCursoRepartidores {
    pub fn new(pedidos: Vec<InfoRepartidorPedido>) -> Self {
        Self { pedidos }
    }

    pub fn pedidos(&self) -> &Vec<InfoRepartidorPedido> {
        &self.pedidos
    }
}
