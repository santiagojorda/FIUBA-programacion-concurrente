use actix::prelude::*;
use common::payment_messages::{AuthorizationResponse, PaymentRequest, PaymentResponse};
use common::tcp_sender::{TcpMessage, TcpSender};
use rand::Rng;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{split, AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::wrappers::LinesStream;

const PAYMENT_APROVAL_RATE: u8 = 7;

/// This actor represents the payment gateway.
/// Its responsibility is to handle payment requests and authorize/reject them.
pub struct PaymentGatewayActor {
    tcp_sender: Arc<Addr<TcpSender>>,
    pub addr: SocketAddr,
}

impl Actor for PaymentGatewayActor {
    type Context = Context<Self>;
}

impl PaymentGatewayActor {
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Addr<Self> {
        PaymentGatewayActor::create(|ctx| {
            let (r_half, w_half) = split(stream);
            PaymentGatewayActor::add_stream(LinesStream::new(BufReader::new(r_half).lines()), ctx);
            let write = Some(w_half);
            let sender_actor = TcpSender { write }.start();
            let tcp_sender = Arc::new(sender_actor);

            PaymentGatewayActor { tcp_sender, addr }
        })
    }

    pub async fn start(addr: SocketAddr) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        loop {
            match listener.accept().await {
                Ok((stream, client_addr)) => {
                    println!("[{}] Connection received from {:?}", addr, client_addr);
                    PaymentGatewayActor::new(stream, client_addr);
                }
                Err(e) => {
                    println!("[{}] Failed to accept connection: {:?}", addr, e);
                }
            }
        }
    }
}

impl StreamHandler<Result<String, tokio::io::Error>> for PaymentGatewayActor {
    fn handle(&mut self, line: Result<String, tokio::io::Error>, ctx: &mut Context<Self>) {
        let tcp_sender = self.tcp_sender.clone();
        let addr = self.addr;
        if let Ok(data) = line {
            if let Ok(message) = serde_json::from_str::<PaymentRequest>(&data) {
                match message {
                    PaymentRequest::CheckPaymentAuthorization(auth_msg) => {
                        let mut rng = rand::thread_rng();
                        let authorized = rng.gen_range(0..=10) <= PAYMENT_APROVAL_RATE;
                        let response = AuthorizationResponse {
                            passenger_id: auth_msg.passenger_id.clone(),
                            authorized,
                        };

                        println!(
                            "[PAYMENT GATEWAY] Payment [{}] for passenger [{}]",
                            if authorized { "authorized" } else { "rejected" },
                            auth_msg.passenger_id
                        );

                        if let Ok(serialized_message) = serde_json::to_string(&response) {
                            tcp_sender
                                .send(TcpMessage(serialized_message))
                                .into_actor(self)
                                .then(move |result, _, _| {
                                    if let Err(err) = result {
                                        eprintln!(
                                            "[{:?}] Failed to send AuthorizationResponse to {}: {:?}",
                                            addr, auth_msg.passenger_id, err
                                        );
                                    } else {
                                        println!(
                                            "[{:?}] Sent AuthorizationResponse to {}",
                                            addr, auth_msg.passenger_id
                                        );
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx);
                        } else {
                            eprintln!("[{:?}] Failed to serialize AuthorizationResponse", addr);
                        }
                    }
                    PaymentRequest::MakePayment(payment_msg) => {
                        let response = PaymentResponse::PaymentDone;
                        if let Ok(serialized_message) = serde_json::to_string(&response) {
                            tcp_sender
                                .send(TcpMessage(serialized_message))
                                .into_actor(self)
                                .then(move |result, _, _| {
                                    if let Err(err) = result {
                                        eprintln!(
                                            "[{:?}] Failed to send PaymentDone to {}: {:?}",
                                            addr, payment_msg.passenger_id, err
                                        );
                                    } else {
                                        println!(
                                            "[{:?}] Sent PaymentDone to {}",
                                            addr, payment_msg.passenger_id
                                        );
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx);
                        } else {
                            eprintln!("[{:?}] Failed to serialize PaymentDone", addr);
                        }
                    }
                }
            } else {
                eprintln!("[{:?}] Failed to deserialize message", addr);
            }
        } else if let Err(err) = line {
            eprintln!("[{:?}] Error reading line: {}", addr, err);
        }
    }
}
