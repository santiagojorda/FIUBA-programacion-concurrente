use super::logs::log_trips;
use crate::admin_actor::admin::Admin;
use crate::coordinator_actor::coordinator_messages::HandleTrip;
use crate::storage_actor::storage_messages::GetPassenger;
use actix::{ActorFutureExt, Handler, ResponseActFuture, WrapFuture};
use common::messages::{AuthConfirmation, RejectTrip, SendPaymentMessage};
use common::payment_messages::{
    AuthorizationResponse, CheckPaymentAuthorization, MakePayment, PaymentMessageType,
    PaymentRequest, PaymentResponse,
};
use common::tcp_sender::TcpMessage;
use rand::random;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

impl Handler<AuthConfirmation> for Admin {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: AuthConfirmation, _ctx: &mut Self::Context) -> Self::Result {
        let passenger_id = msg.passenger_id_ac;
        let coordinator = self.coordinator.clone();
        let address = self.addr;
        let storage_actor = self.storage_addr.clone();

        Box::pin(
            async move {
                if msg.is_authorized {
                    log_trips(format!(
                        "[{:?}] Passenger {} is authorized",
                        address, passenger_id
                    ));
                    if let Err(err) = coordinator
                        .send(HandleTrip {
                            passenger_id_ht: msg.passenger_id_ac,
                        })
                        .await
                    {
                        eprintln!(
                            "[{:?}] Failed to send HandleTrip to {}: {:?}",
                            address, passenger_id, err
                        );
                    } else {
                        log_trips(format!("[{:?}] Sent HandleTrip", address));
                    }
                } else {
                    log_trips(format!(
                        "[{:?}] Passenger {} is not authorized",
                        address, passenger_id
                    ));

                    if let Ok(Some(rejected_passenger)) =
                        storage_actor.send(GetPassenger { id: passenger_id }).await
                    {
                        let tcp_message = match serde_json::to_string(&RejectTrip {
                            response: "Trip Rejected due to inssuficient funds".to_owned(),
                        }) {
                            Ok(json_string) => TcpMessage(json_string),
                            Err(err) => {
                                eprintln!("Error serializing RejectTrip: {}", err);
                                TcpMessage("Error".to_string())
                            }
                        };

                        if let Some(sender) = rejected_passenger.passenger_sender.as_ref() {
                            sender
                                .send(tcp_message)
                                .await
                                .expect("Failed to send RejectTrip");
                        }
                    }
                }
            }
            .into_actor(self)
            .map(|_, _, _| ()),
        )
    }
}

pub async fn get_payment_response(msg: SendPaymentMessage) -> bool {
    let gateway_addr = format!(
        "{}:{}",
        crate::utils::consts::PAYMENT_GATEWAY_IP,
        crate::utils::consts::PAYMENT_GATEWAY_PORT
    );
    let tcp_stream = TcpStream::connect(&gateway_addr).await;
    match tcp_stream {
        Ok(s) => {
            let (reader, mut writer) = s.into_split();

            let message = match msg.message_type {
                PaymentMessageType::Check => {
                    PaymentRequest::CheckPaymentAuthorization(CheckPaymentAuthorization {
                        passenger_id: msg.passenger_id.clone(),
                        amount: msg.amount,
                    })
                }
                PaymentMessageType::Pay => PaymentRequest::MakePayment(MakePayment {
                    passenger_id: msg.passenger_id.clone(),
                    amount: msg.amount,
                }),
            };

            if let Ok(serialized) = serde_json::to_string(&message) {
                let serialized = format!("{}\n", serialized);
                if writer.write_all(serialized.as_bytes()).await.is_err() {
                    log_trips("Failed to send payment message".to_string());
                    return false;
                }
            }
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            if let Ok(Ok(bytes_read)) =
                timeout(Duration::from_secs(5), reader.read_line(&mut line)).await
            {
                if bytes_read > 0 {
                    if let Ok(auth_response) = serde_json::from_str::<AuthorizationResponse>(&line)
                    {
                        return auth_response.authorized;
                    }
                    if let Ok(_payment_response) = serde_json::from_str::<PaymentResponse>(&line) {
                        log_trips(format!("Payment successful for {}", msg.passenger_id));
                        return true;
                    }
                }
            }
            log_trips("Failed to receive a valid response from Payment Gateway".to_string());
            false
        }
        Err(_) => {
            log_trips("Failed to connect to Payment Gateway".to_string());
            false
        }
    }
}

pub fn make_payment_check_message(passenger_id: String) -> SendPaymentMessage {
    SendPaymentMessage {
        passenger_id,
        amount: random::<f32>() * 100.0,
        message_type: PaymentMessageType::Check,
    }
}

pub fn make_payment_done_message(passenger_id: String) -> SendPaymentMessage {
    SendPaymentMessage {
        passenger_id,
        amount: random::<f32>() * 100.0,
        message_type: PaymentMessageType::Pay,
    }
}
