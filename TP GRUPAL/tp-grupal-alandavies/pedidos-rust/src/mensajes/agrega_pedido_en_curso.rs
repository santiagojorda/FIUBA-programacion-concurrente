use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt};
use crate::mensajes::info_restaurante_pedido::InfoRestaurantePedido;

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
pub struct AgregarPedidoEnCurso {
    pedido:  InfoRestaurantePedido
}

impl fmt::Display for AgregarPedidoEnCurso {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AgregarPedidoEnCurso(info pedido: {:?})",
            self.pedido
        )
    }
}

impl AgregarPedidoEnCurso {
    pub fn new(
        pedido:  InfoRestaurantePedido

    ) -> Self {
        Self {
            pedido
        }
    }


    pub fn pedido(&self) -> InfoRestaurantePedido {
        self.pedido.clone()
    }

}
