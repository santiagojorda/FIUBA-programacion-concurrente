use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};

use crate::estructuras_aux::{pedido::Pedido, ubicacion::Ubicacion};

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
#[rtype(result = "()")]
pub struct InfoRepartidorPedido {
    timestamp: i64,
    id_pedido: u32,
    dir_repartidor: SocketAddrV4,
    dir_comensal: SocketAddrV4,
    pedido: Pedido,
    info_restaurante: Option<(u32, Ubicacion)>,
}

impl fmt::Display for InfoRepartidorPedido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InfoRepartidorPedido(timestamp: {}, dir_repartidor: {})",
            self.timestamp, self.dir_repartidor
        )
    }
}

impl InfoRepartidorPedido {
    pub fn new(
        timestamp: i64,
        id_pedido: u32,
        dir_repartidor: SocketAddrV4,
        dir_comensal: SocketAddrV4,
        pedido: Pedido,
        info_restaurante: Option<(u32, Ubicacion)>,
    ) -> Self {
        Self {
            timestamp,
            id_pedido,
            dir_repartidor,
            dir_comensal,
            pedido,
            info_restaurante,
        }
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }

    pub fn dir_repartidor(&self) -> SocketAddrV4 {
        self.dir_repartidor
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
