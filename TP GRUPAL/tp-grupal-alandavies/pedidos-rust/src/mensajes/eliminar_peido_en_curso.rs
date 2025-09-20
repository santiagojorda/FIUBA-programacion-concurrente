use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt};


#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
pub struct EliminarPedidoEnCurso {
    id_pedido: u32,
    id_restaurante: u32,
}

impl fmt::Display for EliminarPedidoEnCurso {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EliminarPeidoEnCurso(restaurante id: {}, peido: {})",
            self.id_pedido, self.id_pedido
        )
    }
}

impl EliminarPedidoEnCurso {
    pub fn new(
        id_pedido: u32,
        id_restaurante: u32,
    ) -> Self {
        Self {
            id_pedido,
            id_restaurante,
        }
    }


    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }
    pub fn id_restaurante(&self) -> u32 {
        self.id_restaurante
    }
}
