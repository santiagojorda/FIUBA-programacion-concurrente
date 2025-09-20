use crate::estructuras_aux::ubicacion::Ubicacion;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct VerRestaurantesInterno {
    id_comensal: u32,
    pub ubicacion_comensal: Ubicacion,
}

impl fmt::Display for VerRestaurantesInterno {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VerRestaurantesInterno(ubicacion: {:?})",
            self.ubicacion_comensal
        )
    }
}

impl VerRestaurantesInterno {
    pub fn new(id_comensal: u32, ubicacion_comensal: Ubicacion) -> Self {
        Self {
            id_comensal,
            ubicacion_comensal,
        }
    }

    pub fn ubicacion_comensal(&self) -> &Ubicacion {
        &self.ubicacion_comensal
    }

    pub fn id_comensal(&self) -> u32 {
        self.id_comensal
    }
}
