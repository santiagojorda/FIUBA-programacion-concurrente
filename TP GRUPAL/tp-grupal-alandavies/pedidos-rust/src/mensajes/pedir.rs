use crate::estructuras_aux::ubicacion::Ubicacion;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddrV4};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct Pedir {
    pub id_pedido: u32,
    pub monto: f32,
    pub id_restaurante: u32,
    pub ubicacion_comensal: Ubicacion,
    pub dir_comensal: SocketAddrV4,
}

impl fmt::Display for Pedir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Pedir(id: {}, monto: {}, restaurante: {}, ubicacion: {:?}, dir_comensal: {:?})",
            self.id_pedido,
            self.monto,
            self.id_restaurante,
            self.ubicacion_comensal,
            self.dir_comensal
        )
    }
}

impl Pedir {
    pub fn new(
        id_pedido: u32,
        monto: f32,
        id_restaurante: u32,
        ubicacion_comensal: Ubicacion,
        dir_comensal: SocketAddrV4,
    ) -> Self {
        Self {
            id_pedido,
            monto,
            id_restaurante,
            ubicacion_comensal,
            dir_comensal,
        }
    }

    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }

    pub fn monto(&self) -> f32 {
        self.monto
    }

    pub fn id_restaurante(&self) -> u32 {
        self.id_restaurante
    }

    pub fn ubicacion_comensal(&self) -> &Ubicacion {
        &self.ubicacion_comensal
    }

    pub fn dir_comensal(&self) -> &SocketAddrV4 {
        &self.dir_comensal
    }
}
