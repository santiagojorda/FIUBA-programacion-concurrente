use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct KeepAlive {
    dir_emisor: SocketAddrV4,
}

impl fmt::Display for KeepAlive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeepAlive from {}", self.dir_emisor)
    }
}

impl KeepAlive {
    pub fn new(dir_emisor: SocketAddrV4) -> Self {
        Self { dir_emisor }
    }

    pub fn dir_emisor(&self) -> SocketAddrV4 {
        self.dir_emisor
    }
}
