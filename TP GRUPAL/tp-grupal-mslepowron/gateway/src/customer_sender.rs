use crate::gateway::Gateway;
use crate::messages::{AuthPaymentResponse, PaymentConfirmed};
use actix::{Actor, AsyncContext, Context, Handler, Message, StreamHandler};
use actix_async_handler::async_handler;
use app_utils::utils::{deserialize_payload, deserialize_tcp_message, serialize_message};
use colored::Colorize;
use log::{error, info};
use std::io::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;

#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpMessage(pub String);

impl TcpMessage {
    pub fn new<S: Into<String>>(string: S) -> Self {
        let mut string = string.into();
        if !string.ends_with('\n') {
            string.push('\n');
        }
        TcpMessage(string)
    }
}
pub struct TcpSender {
    pub write: Option<WriteHalf<TcpStream>>,
    pub addr: SocketAddr,
}

impl Actor for TcpSender {
    type Context = Context<Self>;
}

///el Sender de un customer se usa para comunicarse con el servidor.
/// su tarea principal es convertir mensajes de tipo String en TcpMessage, serialziarlos y enviarlos al servidor.
#[async_handler]
impl Handler<TcpMessage> for TcpSender {
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

impl StreamHandler<Result<String, Error>> for TcpSender {
    fn handle(&mut self, msg: Result<String, Error>, ctx: &mut Context<Self>) {
        if let Ok(line) = msg {
            let line = line.trim();

            let (title, payload) = deserialize_tcp_message(&line);
            match title.as_str() {
                "prepare_payment" => {
                    info!(
                        "{}",
                        format!("[GATEWAY]: Se recibio una solicitud de autorizacion de pago")
                            .yellow()
                    );
                    let payment_authorization = Gateway::authorize_payment();
                    let serialized_msg = serialize_message(
                        "auth_payment_response",
                        AuthPaymentResponse {
                            accepted: payment_authorization,
                        },
                    );

                    if let Err(e) = ctx.address().try_send(TcpMessage::new(serialized_msg)) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
                "abort" => {
                    //se recibio un abort despues de 2pc, asi que se denega el pago
                    let serialized_msg = serialize_message("payment_denied", ());
                    if let Err(e) = ctx.address().try_send(TcpMessage::new(serialized_msg)) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
                "commit_payment" => {
                    let Ok(customer_id) = deserialize_payload::<u64>(&payload, "customer_id")
                    else {
                        error!("Error al deserializar la respuesta de autorizacion del gateway.");
                        return;
                    };
                    let Ok(amount) = deserialize_payload::<f64>(&payload, "amount") else {
                        error!("Error al deserializar la respuesta de autorizacion del gateway.");
                        return;
                    };
                    info!(
                        "{}",
                        format!(
                            "[GATEWAY]:Se ha confirmado un pago del cliente: {}, por: ${:.2}",
                            customer_id, amount
                        )
                        .green()
                    );

                    let serialized_msg =
                        serialize_message("payment_confirmed", PaymentConfirmed { amount });
                    if let Err(e) = ctx.address().try_send(TcpMessage::new(serialized_msg)) {
                        error!("Error al enviar el mensaje: {}", e);
                    }
                }
                _ => {
                    error!("[GATEWAY]: Mensaje {title} invalido enviado al gateway de pagos.");
                }
            }
        }
    }
}
