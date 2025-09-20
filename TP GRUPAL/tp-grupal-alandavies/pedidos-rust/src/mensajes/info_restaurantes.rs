use crate::estructuras_aux::ubicacion::Ubicacion;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct InfoRestaurantes {
    info_restaurantes: HashMap<u32, Ubicacion>, // id_restaurante -> Ubicacion
}

impl fmt::Display for InfoRestaurantes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InfoRestaurantes(info_restaurantes: {:?})",
            self.info_restaurantes
        )
    }
}

impl InfoRestaurantes {
    pub fn new(info_restaurantes: HashMap<u32, Ubicacion>) -> Self {
        Self { info_restaurantes }
    }

    pub fn info_restaurantes(&self) -> &HashMap<u32, Ubicacion> {
        &self.info_restaurantes
    }
}
