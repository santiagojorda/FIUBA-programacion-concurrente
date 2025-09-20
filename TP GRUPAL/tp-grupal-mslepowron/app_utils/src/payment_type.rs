use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};

/// Estructura que representa la forma de pago, ya sea en efectivo o con tarjeta de crédito de un cliente
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentType {
    Cash,
    CreditCard,
}

impl Display for PaymentType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            PaymentType::Cash => write!(f, "cash"),
            PaymentType::CreditCard => write!(f, "creditcard"),
        }
    }
}
