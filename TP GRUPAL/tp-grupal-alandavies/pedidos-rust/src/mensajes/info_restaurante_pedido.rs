use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};


#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
pub struct InfoRestaurantePedido {
    id_pedido: u32,
    dir_comensal: SocketAddrV4,
    id_restaurante: u32,
}

impl fmt::Display for InfoRestaurantePedido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InfoRestaurantePedido(id: {}, dir_comensal: {})",
            self.id_pedido, self.dir_comensal
        )
    }
}

impl InfoRestaurantePedido {
    pub fn new(
        id_pedido: u32,
        dir_comensal: SocketAddrV4,
        id_restaurante: u32,
    ) -> Self {
        Self {
            id_pedido,
            dir_comensal,
            id_restaurante,
        }
    }


    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }

    pub fn dir_comensal(&self) -> SocketAddrV4 {
        self.dir_comensal
    }

    pub fn id_restaurante(&self) -> u32 {
        self.id_restaurante
    }
}
