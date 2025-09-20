use crate::estructuras_aux::ubicacion::Ubicacion;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "Ubicacion")]
pub struct ObtenerUbicacion;

impl fmt::Display for ObtenerUbicacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObtenerUbicacion")
    }
}
impl ObtenerUbicacion {
    pub fn new() -> Self {
        ObtenerUbicacion
    }
}

impl Default for ObtenerUbicacion {
    fn default() -> Self {
        Self::new()
    }
}
