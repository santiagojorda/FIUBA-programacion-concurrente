use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct ActualizarSiguiente;

impl fmt::Display for ActualizarSiguiente {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ActualizarSiguiente")
    }
}

impl ActualizarSiguiente {
    pub fn new() -> Self {
        ActualizarSiguiente
    }
}

impl Default for ActualizarSiguiente {
    fn default() -> Self {
        Self::new()
    }
}
