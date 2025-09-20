use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct InfoLider {
    lider: SocketAddrV4,
}

impl fmt::Display for InfoLider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InfoLider(lider: {})", self.lider)
    }
}

impl InfoLider {
    pub fn new(lider: SocketAddrV4) -> Self {
        Self { lider }
    }

    pub fn lider(&self) -> &SocketAddrV4 {
        &self.lider
    }
}
