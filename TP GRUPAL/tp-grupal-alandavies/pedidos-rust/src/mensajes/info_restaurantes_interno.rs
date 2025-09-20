use crate::estructuras_aux::ubicacion::Ubicacion;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, net::SocketAddrV4};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct InfoRestaurantesInterno {
    pub comensal: SocketAddrV4,
    info_restaurantes: HashMap<u32, Ubicacion>, // id_restaurante -> Ubicacion
    ubicacion_comensal: Ubicacion,
}

impl fmt::Display for InfoRestaurantesInterno {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InfoRestaurantesInterno(info_restaurantes: {:?}, ubicacion_comensal {:?})",
            self.info_restaurantes, self.ubicacion_comensal
        )
    }
}

impl InfoRestaurantesInterno {
    pub fn new(
        comensal: SocketAddrV4,
        info_restaurantes: HashMap<u32, Ubicacion>,
        ubicacion_comensal: Ubicacion,
    ) -> Self {
        Self {
            comensal,
            info_restaurantes,
            ubicacion_comensal,
        }
    }

    pub fn info_restaurantes(&self) -> &HashMap<u32, Ubicacion> {
        &self.info_restaurantes
    }

    pub fn ubicacion_comensal(&self) -> &Ubicacion {
        &self.ubicacion_comensal
    }

    pub fn address_comensal(&self) -> &SocketAddrV4 {
        &self.comensal
    }
}
