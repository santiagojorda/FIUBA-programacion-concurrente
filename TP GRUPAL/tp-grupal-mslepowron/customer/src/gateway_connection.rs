use crate::{
    messages::{AuthPaymentResponse, PaymentConfirmed, PaymentDenied},
    sender::TcpMessage,
};
use actix::{Actor, Addr, Context, Handler, Message, StreamHandler};
use actix_async_handler::async_handler;
use app_utils::utils::{deserialize_payload, deserialize_tcp_message};
use log::error;
use std::io::Error;
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
};

use crate::{customer::Customer, messages::SetCustomer};

#[derive(Message)]
#[rtype(result = "()")]
pub struct _CloseConnection;

pub struct GatewayConnection {
    pub write: Option<WriteHalf<TcpStream>>,
    pub customer_addr: Option<Addr<Customer>>,
}

impl Actor for GatewayConnection {
    type Context = Context<Self>;
}

#[async_handler]
impl Handler<TcpMessage> for GatewayConnection {
    type Result = ();

    async fn handle(&mut self, msg: TcpMessage, _ctx: &mut Context<Self>) {
        let mut writer = self.write.take().expect("Error al escribir");

        let ret = async move {
            writer
                .write_all(msg.0.as_bytes())
                .await
                .expect("Error al escribir");
            writer
        }
        .await;
        self.write = Some(ret);
    }
}

#[async_handler]
impl Handler<SetCustomer> for GatewayConnection {
    type Result = ();

    fn handle(&mut self, msg: SetCustomer, _ctx: &mut Self::Context) -> Self::Result {
        self.customer_addr = Some(msg.customer);
    }
}

impl StreamHandler<Result<String, Error>> for GatewayConnection {
    fn handle(&mut self, msg: Result<String, Error>, _ctx: &mut Context<Self>) {
        if let Ok(line) = msg {
            let line = line.trim();

            let (title, payload) = deserialize_tcp_message(&line);
            match title.as_str() {
                //esto es lo que recibe del gateway
                "auth_payment_response" => {
                    if let Some(customer) = &self.customer_addr {
                        let Ok(payment_response) =
                            deserialize_payload::<bool>(&payload, "accepted")
                        else {
                            error!(
                                "Error al deserializar la respuesta de autorizacion del gateway."
                            );
                            return;
                        };
                        let response = AuthPaymentResponse {
                            accepted: payment_response,
                        };

                        let _ = customer.try_send(response);
                    }
                }
                "payment_confirmed" => {
                    if let Some(customer) = &self.customer_addr {
                        let Ok(amount) = deserialize_payload::<f64>(&payload, "amount") else {
                            error!(
                                "Error al deserializar la respuesta de autorizacion del gateway."
                            );
                            return;
                        };
                        let payment_status = PaymentConfirmed { amount };

                        let _ = customer.try_send(payment_status);
                    }
                }
                "payment_denied" => {
                    if let Some(customer) = &self.customer_addr {
                        let payment_status = PaymentDenied {};

                        let _ = customer.try_send(payment_status);
                    }
                }
                _ => {
                    error!("[CUSTOMER]: Mensaje {title} invalido");
                }
            }
        }
    }
}
