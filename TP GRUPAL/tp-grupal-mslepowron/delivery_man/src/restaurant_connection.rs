use crate::delivery::DeliveryWorker;
use crate::messages::{RequestDelivery, RestaurantConnectionMessage};
use actix::{Actor, Addr, AsyncContext, Context, Handler, StreamHandler};
use actix_async_handler::async_handler;
use app_utils::utils::{deserialize_payload, deserialize_tcp_message};
use log::{debug, error};
use std::io::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;

pub struct RestaurantConnection {
    pub write: Option<WriteHalf<TcpStream>>,
    pub addr: SocketAddr,
    pub delivery_addr: Addr<DeliveryWorker>,
}

impl Actor for RestaurantConnection {
    type Context = Context<Self>;
}

#[async_handler]
impl Handler<RestaurantConnectionMessage> for RestaurantConnection {
    type Result = ();

    async fn handle(
        &mut self,
        msg: RestaurantConnectionMessage,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        //let message = msg.0.clone();
        let mut write = self.write.take().expect(
            "No debería poder llegar otro mensaje antes de que vuelva por usar AtomicResponse",
        );

        let ret = async move {
            write
                .write_all(msg.0.as_bytes())
                .await
                .expect("Error al escribir");
            write
        }
        .await;

        self.write = Some(ret);
    }
}

impl StreamHandler<Result<String, Error>> for RestaurantConnection {
    fn handle(&mut self, item: Result<String, Error>, ctx: &mut Self::Context) {
        if let Ok(raw) = item {
            let line = raw.trim();
            debug!("[DELIVERY-CONN] RAW: {}", line);

            // parsea JSON {"title": "...", "payload": {...}}
            let (title, payload) = deserialize_tcp_message(line);
            debug!("[DELIVERY-CONN] title=`{}` payload=`{}`", title, payload);

            if title == "request_delivery" {
                // 1) parsea los campos desde el payload
                let order_id: u64 = deserialize_payload(&payload, "order_id")
                    .expect("order_id inválido en request_delivery");
                let restaurant_position: (u64, u64) =
                    deserialize_payload(&payload, "restaurant_position")
                        .expect("restaurant_position inválido");
                let customer_position: (u64, u64) =
                    deserialize_payload(&payload, "customer_position")
                        .expect("customer_position inválido");
                let customer_socket: String = deserialize_payload(&payload, "customer_socket")
                    .expect("customer_socket inválido");

                // info!(
                //     "[DELIVERY-CONN] Parsed → order={}, rest@{:?}, cust@{:?}",
                //     order_id, restaurant_position, customer_position
                // );

                // 2) reenvía al DeliveryWorker
                let req = RequestDelivery {
                    id_order: order_id,
                    restaurant_position,
                    customer_position,
                    customer_socket,
                    addr: ctx.address(), // Addr<RestaurantConnection>
                };
                self.delivery_addr.do_send(req);
            } else {
                error!("[DELIVERY-CONN] Mensaje '{}' no reconocido", title);
            }
        }
    }
}
