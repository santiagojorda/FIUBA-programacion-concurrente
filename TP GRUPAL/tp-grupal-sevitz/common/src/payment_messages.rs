use actix::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub enum PaymentMessageType {
    Check,
    Pay,
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub enum PaymentRequest {
    CheckPaymentAuthorization(CheckPaymentAuthorization),
    MakePayment(MakePayment),
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct CheckPaymentAuthorization {
    pub passenger_id: String,
    pub amount: f32,
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct AuthorizationResponse {
    pub passenger_id: String,
    pub authorized: bool,
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct MakePayment {
    pub passenger_id: String,
    pub amount: f32,
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub enum PaymentResponse {
    PaymentDone,
    PaymentError(String),
}
