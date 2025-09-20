use crate::customer_sender::TcpSender;
use actix::Addr;
use actix::Message;
use serde::Serialize;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct NewCustomer {
    pub addr: Addr<TcpSender>,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct CustomerLogin {
    pub name: String,
    pub addr: Addr<TcpSender>,
}

#[derive(Message)]
#[rtype(result = "bool")]
pub struct PreparePayment {}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct AuthPaymentResponse {
    pub accepted: bool,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct PaymentConfirmed {
    pub amount: f64,
}
