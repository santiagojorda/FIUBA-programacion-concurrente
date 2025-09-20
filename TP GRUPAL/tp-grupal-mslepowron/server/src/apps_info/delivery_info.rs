use serde::{Deserialize, Serialize};

/// Struct que almacena los datos de un repartidor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryInfo {
    pub id: u64,
    pub position: (u64, u64),
    pub socket: String,
}
