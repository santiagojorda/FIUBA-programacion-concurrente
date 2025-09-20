use serde::Serialize;

/// Enum que representa los diferentes estados de un pedido
#[derive(Serialize, Clone)]
pub enum OrderStatus {
    OrderConfirmed,
    OrderCanceled,
    PreparingOrder,
    OutOfPickup,
    ReadyToPickup,
    OutForDelivery,
    CarryingOrder,
    ArrivedDestination,
    OrderDelivered,
}

/// Estructura que representa un pedido con su ID, precio total, estado y entrega designada. Guarda tambien la direccion al comensal correspondiente
pub struct Order<C> {
    pub order_id: u64,
    pub total_price: f64,
    pub order_status: OrderStatus,
    pub client_connection: C,
    pub customer_position: (u64, u64),
    pub customer_socket: String,
}

#[derive(Serialize, Clone)]
pub struct SerializableOrder {
    pub order_id: u64,
    pub total_price: f64,
    pub order_status: OrderStatus,
    pub customer_position: (u64, u64),
}
