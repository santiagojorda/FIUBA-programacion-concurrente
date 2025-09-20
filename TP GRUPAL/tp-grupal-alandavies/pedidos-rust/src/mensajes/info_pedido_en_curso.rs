use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};

use crate::estructuras_aux::{pedido::Pedido, ubicacion::Ubicacion};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct InfoPedidoEnCurso {
    dir_comensal: SocketAddrV4,
    pedido: Pedido,
    info_restaurante: Option<(u32, Ubicacion)>,
}

impl fmt::Display for InfoPedidoEnCurso {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InfoPedidoEnCurso(dir_comensal: {}, pedido: {:?})",
            self.dir_comensal, self.pedido
        )
    }
}

impl InfoPedidoEnCurso {
    pub fn new(
        dir_comensal: SocketAddrV4,
        pedido: Pedido,
        info_restaurante: Option<(u32, Ubicacion)>,
    ) -> Self {
        Self {
            dir_comensal,
            pedido,
            info_restaurante,
        }
    }

    pub fn dir_comensal(&self) -> SocketAddrV4 {
        self.dir_comensal
    }

    pub fn pedido(&self) -> &Pedido {
        &self.pedido
    }

    pub fn info_restaurante(&self) -> Option<&(u32, Ubicacion)> {
        self.info_restaurante.as_ref()
    }
}
