use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Debug, Deserialize)]
pub struct RestaurantData {
    pub name: String,
    pub socket: String,
    pub id: u64,
    pub position: (u64, u64),
}
