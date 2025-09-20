use crate::{
    logger::log::Logger,
    mensajes::{
        cobrar::Cobrar,
        intento_pago::IntentoPago,
        mcode::{PAGO_AUTORIZADO_CODE, PAGO_RECHAZADO_CODE},
        pago_autorizado::PagoAutorizado,
        pago_rechazado::PagoRechazado,
    },
};
use actix::{Actor, Context, Handler, ResponseActFuture, WrapFuture};
use rand::Rng;
use std::fs::File;
use std::io::Write;
use tokio::net::UdpSocket;

/// Se encarga de gestionar los pagos de los pedidos.
///
/// Campos:
/// - `log_file`: Archivo donde se registran los eventos de pago.
pub struct GatewayPagos {
    log_file: File,
    logger: Logger,
}

impl GatewayPagos {
    pub fn new(log_file: File, logger: Logger) -> Self {
        GatewayPagos { log_file, logger }
    }
}

impl Actor for GatewayPagos {
    type Context = Context<Self>;
}

impl Handler<IntentoPago> for GatewayPagos {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja el mensaje `IntentoPago`, simulando la autorización o rechazo del pago.
    /// Si el pago es rechazado, se envía un mensaje `PagoRechazado` al comensal.
    /// Si el pago es autorizado, se envía un mensaje `PagoAutorizado` al comensal.
    fn handle(&mut self, msg: IntentoPago, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Recibido IntentoPago para pedido {} de comensal {}",
                msg.id_pedido(),
                msg.comensal_ip
            ),
            colored::Color::Cyan,
        );
        let mut rng = rand::thread_rng();
        let rechazar = rng.gen_bool(0.3); // 30% de rechazo

        if rechazar {
            let mensaje = PagoRechazado::new(msg.id_pedido);
            let _ = self.logger.info(
                &format!(
                    "Enviando PagoRechazado para pedido {} de comensal {}",
                    msg.id_pedido(),
                    msg.comensal_ip
                ),
                colored::Color::Red,
            );
            let _ = writeln!(self.log_file, "[PEDIDO {}] PAGO RECHAZADO", msg.id_pedido);
            Box::pin(
                async move {
                    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
                        if let Ok(serialized_msg) = serde_json::to_vec(&mensaje) {
                            let mut data: Vec<u8> = Vec::new();
                            data.push(PAGO_RECHAZADO_CODE);
                            data.extend(serialized_msg.len().to_be_bytes());
                            data.extend(serialized_msg);
                            if socket.send_to(&data, msg.comensal_ip).await.is_err() {
                                eprintln!("Error al enviar el mensaje de pago autorizado");
                            }
                        }
                    }
                }
                .into_actor(self),
            )
        } else {
            let mensaje = PagoAutorizado::new(
                msg.id_pedido,
                msg.monto,
                msg.id_restaurante,
                msg.ubicacion_comensal,
            );
            let _ = self.logger.info(
                &format!(
                    "Enviando PagoAutorizado para pedido {} de comensal {}",
                    msg.id_pedido(),
                    msg.comensal_ip
                ),
                colored::Color::Cyan,
            );
            let _ = writeln!(self.log_file, "[PEDIDO {}] PAGO AUTORIZADO", msg.id_pedido);
            Box::pin(
                async move {
                    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
                        if let Ok(serialized_msg) = serde_json::to_vec(&mensaje) {
                            let mut data: Vec<u8> = Vec::new();
                            data.push(PAGO_AUTORIZADO_CODE);
                            data.extend(serialized_msg.len().to_be_bytes());
                            data.extend(serialized_msg);
                            if socket.send_to(&data, msg.comensal_ip).await.is_err() {
                                eprintln!("Error al enviar el mensaje de pago autorizado");
                            }
                        } else {
                            eprintln!(
                                "Error al serializar el mensaje de pago autorizado: {:?}",
                                mensaje
                            );
                        }
                    } else {
                        eprintln!("Error al crear el socket UDP");
                    }
                }
                .into_actor(self),
            )
        }
    }
}

impl Handler<Cobrar> for GatewayPagos {
    type Result = ();

    /// Maneja el mensaje `Cobrar`, registrando el cobro exitoso en el archivo de log.
    fn handle(&mut self, msg: Cobrar, _ctx: &mut Self::Context) -> Self::Result {
        let _ = writeln!(
            self.log_file,
            "[PEDIDO {}] COBRO EFECTUADO CON ÉXITO",
            msg.id_pedido
        );
        let _ = self.logger.info(
            &format!(" Cobro exitoso para pedido {}", msg.id_pedido),
            colored::Color::Green,
        );
    }
}
