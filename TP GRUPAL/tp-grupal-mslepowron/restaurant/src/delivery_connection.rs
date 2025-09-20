use crate::messages::{AcceptDelivery, TryNextDelivery};
use crate::restaurant::Restaurant;
use crate::sender::TcpMessage;
use actix::{
    Actor, ActorFutureExt, Addr, Context, Handler, ResponseActFuture, StreamHandler, WrapFuture,
};
use app_utils::utils::{deserialize_payload, deserialize_tcp_message};
use colored::Colorize;
use log::{error, info};
use std::io::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;

/// Cada DeliveryConnection gestiona un canal TCP directo al delivery.
pub struct DeliveryConnection {
    /// El WriteHalf para escribir
    pub write: Option<WriteHalf<TcpStream>>,
    /// Actor Restaurante al que notificamos con `AcceptDelivery`
    pub owner: Addr<Restaurant>,
    /// IP:PUERTO del delivery
    pub addr: SocketAddr,
}

impl Actor for DeliveryConnection {
    type Context = Context<Self>;
}

/// Lee líneas del delivery; espera `"ok"` o `"no"`.
impl StreamHandler<Result<String, Error>> for DeliveryConnection {
    fn handle(&mut self, item: Result<String, Error>, _ctx: &mut Context<Self>) {
        if let Ok(raw) = item {
            let line = raw.trim();
            let (title, payload) = deserialize_tcp_message(line);

            match title.as_str() {
                "ok" => {
                    // parsea order_id si lo incluyes en el ok
                    let order_id: u64 =
                        deserialize_payload(&payload, "order_id").unwrap_or_default();
                    let delivery_socket: String = deserialize_payload(&payload, "delivery_socket")
                        .expect("ok sin delivery_socket");
                    let delivery_name: String = deserialize_payload(&payload, "delivery_name")
                        .expect("ok sin delivery_name");
                    info!(
                        "{}",
                        format!(
                            "[RESTUARANT]: El delivery: {} ha aceptado el pedido🚴",
                            delivery_socket.clone()
                        )
                        .magenta()
                    );
                    self.owner.do_send(AcceptDelivery {
                        order_id,
                        delivery_position: (0, 0),
                        delivery_socket,
                        delivery_name,
                    });
                }
                "no" => {
                    info!(
                        "[RESTAURANT]: repartidor {} rechazó el pedido ❌",
                        self.addr
                    );
                    self.owner.do_send(TryNextDelivery);
                }
                other => {
                    error!(
                        "[DELIVERY-CONN {}] mensaje desconocido `{}`",
                        self.addr, other
                    );
                }
            }
        }
    }
}

/// Permite enviar `TcpMessage` al delivery.  
impl Handler<TcpMessage> for DeliveryConnection {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: TcpMessage, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(write_half) = self.write.take() {
            Box::pin(
                async move {
                    let mut w = write_half;
                    w.write_all(msg.0.as_bytes())
                        .await
                        .expect("Error escribiendo al delivery");
                    w
                }
                .into_actor(self)
                .map(|w, actor, _| {
                    actor.write = Some(w);
                }),
            )
        } else {
            Box::pin(async {}.into_actor(self))
        }
    }
}
