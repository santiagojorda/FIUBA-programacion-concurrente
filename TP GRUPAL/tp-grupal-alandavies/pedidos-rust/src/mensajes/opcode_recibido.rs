use actix::Message;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct OpcodeRecibido {
    codigo: u8,
    remitente: SocketAddr,
}

impl OpcodeRecibido {
    pub fn new(codigo: u8, remitente: SocketAddr) -> Self {
        OpcodeRecibido { codigo, remitente }
    }

    pub fn codigo(&self) -> u8 {
        self.codigo
    }

    pub fn remitente(&self) -> &SocketAddr {
        &self.remitente
    }
}
