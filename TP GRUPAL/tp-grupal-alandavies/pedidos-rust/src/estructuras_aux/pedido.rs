use crate::estructuras_aux::estado_pedido::EstadoPedido;
use crate::estructuras_aux::ubicacion::Ubicacion;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pedido {
    id: u32,
    estado: EstadoPedido,
    monto: f32,
    id_restaurante: u32,
    ubicacion_comensal: Ubicacion,
}

impl fmt::Display for Pedido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Pedido {{ id: {}, estado: {:?}, monto: {}, id_restaurante: {}, ubicacion_comensal: {} }}",
            self.id, self.estado, self.monto, self.id_restaurante, self.ubicacion_comensal
        )
    }
}

impl Pedido {
    pub fn new(
        id: u32,
        estado: EstadoPedido,
        monto: f32,
        id_restaurante: u32,
        ubicacion_comensal: Ubicacion,
    ) -> Self {
        Pedido {
            id,
            estado,
            monto,
            id_restaurante,
            ubicacion_comensal,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn estado(&self) -> &EstadoPedido {
        &self.estado
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
}
