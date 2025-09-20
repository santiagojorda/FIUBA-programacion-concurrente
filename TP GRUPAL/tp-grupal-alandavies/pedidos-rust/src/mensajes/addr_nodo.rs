use actix::{Actor, Addr, Message};
use std::fmt;

use crate::mensajes::progreso_pedido::ProgresoPedido;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct AddrNodo<A>
where
    A: Actor + Send + 'static,
{
    addr: Addr<A>,
}

impl<A> fmt::Display for AddrNodo<A>
where
    A: Actor + Send + 'static,
    A: actix::Handler<ProgresoPedido>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AddrNodo({:?})", self.addr)
    }
}

impl<A> AddrNodo<A>
where
    A: Actor + Send + 'static,
{
    pub fn new(addr: Addr<A>) -> Self {
        Self { addr }
    }

    pub fn get_addr(&self) -> &Addr<A> {
        &self.addr
    }
}
