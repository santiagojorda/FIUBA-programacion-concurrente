use crate::estructuras_aux::pedido::Pedido;
use actix::Message;

#[derive(Message)]
#[rtype(result = "()")]
pub struct PedidoAsignadoAVos {
    pedido: Pedido,
}

impl PedidoAsignadoAVos {
    pub fn new(pedido: Pedido) -> Self {
        Self { pedido }
    }

    pub fn pedido(&self) -> &Pedido {
        &self.pedido
    }
}
