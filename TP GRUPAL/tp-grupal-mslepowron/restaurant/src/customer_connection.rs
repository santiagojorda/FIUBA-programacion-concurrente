use crate::messages::{NewOrder, RecoverOrderData, RestaurantOrderResponse};
use crate::restaurant::Restaurant;
use crate::sender::TcpMessage;
use actix::{Actor, Addr, Context, StreamHandler};
use actix::{AsyncContext, Handler};
use actix_async_handler::async_handler;
use app_utils::payment_type::PaymentType;
use app_utils::utils::{deserialize_payload, deserialize_tcp_message, serialize_message};
use colored::Colorize;
use log::{error, info};
use std::io::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;

pub struct CustomerConnection {
    pub write: Option<WriteHalf<TcpStream>>,
    pub addr: SocketAddr,
    pub restaurant_address: Addr<Restaurant>,
}

impl Actor for CustomerConnection {
    type Context = Context<Self>;
}

//necesito un stream handler para recir mensajes de TCP
impl StreamHandler<Result<String, Error>> for CustomerConnection {
    fn handle(&mut self, msg: Result<String, Error>, ctx: &mut Context<Self>) {
        if let Ok(line) = msg {
            //println!("[CUSTOMER_CONNECTION] RAW LINE: {:?}", line);

            let line = line.trim();
            let (title, payload) = deserialize_tcp_message(&line);

            // println!(
            //     "[CUSTOMER_CONNECTION] TITLE: {:?}, PAYLOAD: {:?}",
            //     title, payload
            // );

            match title.as_str() {
                "prepare_order" => {
                    let order_accepted = Restaurant::accept_new_order();

                    let serialized_msg = serialize_message(
                        "prepare_order_response",
                        RestaurantOrderResponse {
                            accepted: order_accepted,
                        },
                    );
                    info!(
                        "{}",
                        format!("[RESTAURANT]: Se recibio una solicitud de pedido").green()
                    );
                    // Le comunica la decision al customer sobre si le va aceptar la order o no..
                    if let Err(e) = ctx.address().try_send(TcpMessage::new(serialized_msg)) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
                "commit_order" => {
                    let customer_id = deserialize_payload::<u64>(&payload, "customer_id")
                        .expect("customer_id inválido");
                    let payment_type = deserialize_payload::<PaymentType>(&payload, "payment_type")
                        .expect("payment_type inválido");
                    let amount =
                        deserialize_payload::<f64>(&payload, "amount").expect("amount inválido");
                    let customer_position =
                        deserialize_payload::<(u64, u64)>(&payload, "customer_position")
                            .expect("customer_position inválido");
                    let customer_socket =
                        deserialize_payload::<String>(&payload, "customer_socket")
                            .expect("customer_socket inválido");
                    info!(
                        "{}",
                        format!("[RESTAURANT]: Nuevo pedido recibido por cliente: {customer_id}")
                            .green()
                    );

                    let new_order = NewOrder {
                        total_price: amount,
                        client_conn: ctx.address(),
                        customer_position,
                        payment_type,
                        customer_socket,
                    };

                    self.restaurant_address.do_send(new_order);
                }
                "abort" => {
                    let serialized_msg = serialize_message("order_canceled", ());
                    if let Err(e) = ctx.address().try_send(TcpMessage::new(serialized_msg)) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
                "recover_customer_order" => {
                    let order_id = deserialize_payload::<u64>(&payload, "order_id")
                        .expect("order_id inválido");
                    if let Err(e) = self.restaurant_address.try_send(RecoverOrderData {
                        order_id,
                        addr: ctx.address(),
                    }) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
                _ => {
                    error!("[RESTAURANT]: Mensaje {title} invalido");
                }
            }
        }
    }
}

#[async_handler]
impl Handler<TcpMessage> for CustomerConnection {
    type Result = ();

    async fn handle(&mut self, msg: TcpMessage, _ctx: &mut Context<Self>) {
        // 1) clonamos el texto para el futuro
        let write_data = msg.0.clone();

        // 2) extraemos el WriteHalf
        let mut writer = self
            .write
            .take()
            .expect("WriteHalf debe estar presente en CustomerConnection");

        // 3) escribimos en un async move idéntico al TcpSender
        let ret = async move {
            writer
                .write_all(write_data.as_bytes())
                .await
                .expect("Error al escribir al cliente");
            writer
        }
        .await;

        // 4) lo volvemos a guardar para próximas escrituras
        self.write = Some(ret);
    }
}
