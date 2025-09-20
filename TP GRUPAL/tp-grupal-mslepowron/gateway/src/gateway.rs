use crate::customer_sender::{TcpMessage, TcpSender};
use crate::messages::PreparePayment;
use crate::messages::{CustomerLogin, NewCustomer};
use actix::{Actor, Addr, Context, Handler};
use app_utils::utils::serialize_message;
use colored::Colorize;
use log::{error, info};
use rand::Rng;
use serde_json::json;
use std::collections::HashMap;

/// Estructura que representa el tipo de pago
pub struct Gateway {
    pub customer_connections: Vec<Addr<TcpSender>>,
    pub customers: HashMap<String, u64>,
    pub customer_id_count: u64,
}

impl Actor for Gateway {
    type Context = Context<Self>;
}

impl Gateway {
    pub fn authorize_payment() -> bool {
        let mut rng = rand::thread_rng();
        rng.gen_bool(0.9) // 90% de aceptar, 10% de rechazar (false)
    }
}

impl Handler<NewCustomer> for Gateway {
    //se conecta un nuevo customer que quiere pagar
    type Result = ();

    fn handle(&mut self, msg: NewCustomer, _ctx: &mut Self::Context) -> Self::Result {
        self.customer_connections.push(msg.addr);
    }
}

impl Handler<CustomerLogin> for Gateway {
    //recibe un request de un customer ya existente, que se quiere conectar a pagar.
    type Result = ();

    fn handle(&mut self, msg: CustomerLogin, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(id) = self.customers.get(&msg.name) {
            // Si el cliente ya existe, se envía el id al cliente
            let message = serialize_message("login_successful", json!({"id": id}));

            if let Err(e) = msg.addr.try_send(TcpMessage(format!("{}\n", message))) {
                error!("Error al enviar el mensaje: {}", e);
            }
        } else {
            // Si el cliente no existe, se crea un nuevo id y se envía al cliente
            self.customer_id_count += 1;
            let new_id = self.customer_id_count;
            self.customers.insert(msg.name.clone(), new_id);

            info!(
                "{}",
                format!(
                    "[GATEWAY]: Nuevo comensal {} con ID {} conectado\n",
                    msg.name, new_id
                )
                .green()
            );
            let message = serialize_message("login_successful", json!({"id": new_id}));

            if let Err(e) = msg.addr.try_send(TcpMessage(format!("{}\n", message))) {
                error!("Error al enviar el mensaje: {}", e);
            }
        }
    }
}

impl Handler<PreparePayment> for Gateway {
    type Result = bool;

    fn handle(&mut self, _msg: PreparePayment, _ctx: &mut Self::Context) -> Self::Result {
        Gateway::authorize_payment()
    }
}
