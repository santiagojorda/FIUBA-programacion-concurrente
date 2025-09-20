use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum EstadoPedido {
    Pendiente,
    Preparando,
    Listo,
    EnCamino,
    Entregado,
    Cancelado,
}

impl fmt::Display for EstadoPedido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EstadoPedido::Pendiente => write!(f, "Pendiente"),
            EstadoPedido::Preparando => write!(f, "Preparando"),
            EstadoPedido::Listo => write!(f, "Listo"),
            EstadoPedido::EnCamino => write!(f, "En Camino"),
            EstadoPedido::Entregado => write!(f, "Entregado"),
            EstadoPedido::Cancelado => write!(f, "Cancelado"),
        }
    }
}
