use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct SolicitarPedidoEnCurso {
    dir_emisor: SocketAddrV4,
}

impl fmt::Display for SolicitarPedidoEnCurso {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SolicitarPedidoEnCurso(dir_emisor: {})", self.dir_emisor)
    }
}

impl SolicitarPedidoEnCurso {
    pub fn new(dir_emisor: SocketAddrV4) -> Self {
        Self { dir_emisor }
    }

    pub fn dir_emisor(&self) -> SocketAddrV4 {
        self.dir_emisor
    }
}
