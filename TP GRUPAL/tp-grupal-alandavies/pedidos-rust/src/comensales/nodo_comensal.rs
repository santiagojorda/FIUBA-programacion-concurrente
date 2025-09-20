use crate::logger::log::Logger;
use crate::mensajes::info_restaurantes::InfoRestaurantes;
use crate::mensajes::intento_pago::IntentoPago;
use crate::mensajes::llego_pedido::LlegoPedido;
use crate::mensajes::mcode::{INTENTO_PAGO_CODE, PEDIR_CODE, VER_RESTAURANTES_CODE};
use crate::mensajes::pago_autorizado::PagoAutorizado;
use crate::mensajes::pago_rechazado::PagoRechazado;
use crate::mensajes::progreso_pedido::ProgresoPedido;
use crate::mensajes::ver_restaurantes::VerRestaurantes;
use crate::mensajes::ver_restaurantes_interno::VerRestaurantesInterno;
use crate::{comensales::comensal::Comensal, mensajes::pedir::Pedir};
use actix::{Actor, Addr, Context, Handler, ResponseActFuture, WrapFuture};
use rand::seq::SliceRandom;
use std::{collections::HashMap, net::SocketAddrV4};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Representa un nodo que maneja las interacciones de un comensal con los restaurantes y el gateway de pagos.
///
/// Campos:
/// - `ip`: Dirección IP del nodo comensal.
/// - `restaurantes`: Mapa de restaurantes disponibles, donde la clave es el ID del restaurante y el valor es su dirección.
/// - `gateway_pagos`: Dirección del gateway de pagos.
/// - `comensal`: Dirección del actor Comensal asociado a este nodo.
pub struct NodoComensal {
    ip: SocketAddrV4,
    restaurantes: HashMap<u32, SocketAddrV4>, // id_restaurante -> direccion
    gateway_pagos: SocketAddrV4,
    comensal: Addr<Comensal>,
    logger: Logger,
}

impl NodoComensal {
    pub fn new(
        ip: SocketAddrV4,
        restaurantes: HashMap<u32, SocketAddrV4>,
        gateway_pagos: SocketAddrV4,
        comensal: Addr<Comensal>,
        logger: Logger,
    ) -> Self {
        NodoComensal {
            ip,
            restaurantes,
            gateway_pagos,
            comensal,
            logger,
        }
    }
}

impl Actor for NodoComensal {
    type Context = Context<Self>;
}

impl Handler<Pedir> for NodoComensal {
    type Result = ResponseActFuture<Self, ()>;

    /// Al recibir un pedido del comensal, envía un intento de pago al gateway de pagos.
    fn handle(&mut self, msg: Pedir, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info("Recibo Pedir", colored::Color::Cyan);

        let intento_pago = IntentoPago::new(
            msg.id_pedido,
            msg.monto,
            self.ip,
            msg.id_restaurante,
            msg.ubicacion_comensal,
        );

        let gateway_pagos = self.gateway_pagos;

        let logger = self.logger.clone();

        Box::pin(
            async move {
                if let Ok(mut stream) = TcpStream::connect(gateway_pagos).await {
                    if let Ok(data) = serde_json::to_vec(&intento_pago) {
                        let _ = stream.write_u8(INTENTO_PAGO_CODE).await;
                        let _ = stream.write_u32(data.len() as u32).await;
                        let _ = stream.write_all(&data).await;

                        let _ = logger.info(
                            &format!(
                                "IntentoPago enviado al Gateway de Pagos en {}",
                                gateway_pagos
                            ),
                            colored::Color::Cyan,
                        );
                    } else {
                        println!("Error al serializar IntentoPago");
                    }
                } else {
                    let _ = logger.error(&format!(
                        "No me pude conectar con pagos en {}",
                        gateway_pagos
                    ));
                }
            }
            .into_actor(self),
        )
    }
}

impl Handler<VerRestaurantesInterno> for NodoComensal {
    type Result = ResponseActFuture<Self, ()>;

    /// Al recibir un mensaje de ver restaurantes de parte del comensal, se intenta comunicar con algún nodo restaurante al azar y le envía la solicitud de ver restaurantes.
    fn handle(&mut self, msg: VerRestaurantesInterno, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            "Recibo VerRestaurantesInterno del comensal",
            colored::Color::Cyan,
        );

        let mut direcciones: Vec<SocketAddrV4> = self.restaurantes.values().cloned().collect();
        let mut rng = rand::thread_rng();
        direcciones.shuffle(&mut rng);

        let ver_msg = VerRestaurantes::new(self.ip, msg.ubicacion_comensal);

        let logger = self.logger.clone();

        Box::pin(
            async move {
                for direccion in direcciones {
                    if let Ok(mut stream) = TcpStream::connect(direccion).await {
                        if let Ok(data) = serde_json::to_vec(&ver_msg) {
                            let _ = stream.write_u8(VER_RESTAURANTES_CODE).await;
                            let _ = stream.write_u32(data.len() as u32).await;
                            let _ = stream.write_all(&data).await;
                            let _ = logger.info(
                                &format!("VerRestaurantes enviado a {}", direccion),
                                colored::Color::Cyan,
                            );
                            break;
                        } else {
                            println!("Error al serializar VerRestaurantes");
                        }
                    }
                }
            }
            .into_actor(self),
        )
    }
}

impl Handler<InfoRestaurantes> for NodoComensal {
    type Result = ResponseActFuture<Self, ()>;

    /// Al recibir información de restaurantes, la reenvía al comensal asociado.
    fn handle(&mut self, msg: InfoRestaurantes, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            "Recibo InfoRestaurantes del NodoRestaurante",
            colored::Color::Cyan,
        );
        let comensal = self.comensal.clone();

        let _ = self
            .logger
            .info("Reenvío InfoRestaurantes al Comensal", colored::Color::Cyan);
        Box::pin(
            async move {
                comensal.do_send(msg);
            }
            .into_actor(self),
        )
    }
}

impl Handler<ProgresoPedido> for NodoComensal {
    type Result = ResponseActFuture<Self, ()>;

    /// Lo recibe cuando el pedido ha sido cancelado o finalizado exitosamente.
    /// Reenvía el progreso del pedido al comensal asociado para que pueda realizar un nuevo pedido.
    fn handle(&mut self, msg: ProgresoPedido, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Recibo ProgresoPedido con estado {} del NodoRestaurante",
                msg.estado_pedido()
            ),
            colored::Color::Cyan,
        );
        let comensal = self.comensal.clone();

        let _ = self
            .logger
            .info("Reenvío ProgresoPedido al Comensal", colored::Color::Cyan);

        Box::pin(
            async move {
                comensal.do_send(msg);
            }
            .into_actor(self),
        )
    }
}

impl Handler<PagoRechazado> for NodoComensal {
    type Result = ResponseActFuture<Self, ()>;

    /// Si el pago fue rechazado, se lo reenvía al Comensal para que pueda vuelver a pedir.
    fn handle(&mut self, msg: PagoRechazado, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            "Recibo PagoRechazado del Gateway de Pagos",
            colored::Color::Cyan,
        );

        let comensal = self.comensal.clone();
        let _ = self
            .logger
            .info("Reenvío PagoRechazado al Comensal", colored::Color::Cyan);

        Box::pin(
            async move {
                comensal.do_send(msg);
            }
            .into_actor(self),
        )
    }
}

impl Handler<PagoAutorizado> for NodoComensal {
    type Result = ResponseActFuture<Self, ()>;

    /// Al recibir un pago autorizado, se envía un mensaje de Pedir a uno de los restaurantes disponibles.
    fn handle(&mut self, msg: PagoAutorizado, _ctx: &mut Self::Context) -> Self::Result {
        let mut direcciones: Vec<SocketAddrV4> = self.restaurantes.values().cloned().collect();
        let mut rng = rand::thread_rng();
        direcciones.shuffle(&mut rng);

        let _ = self.logger.info(
            "Recibo PagoAutorizado del Gateway de Pagos",
            colored::Color::Cyan,
        );

        let pedir_msg = Pedir::new(
            msg.id_pedido(),
            msg.monto,
            msg.id_restaurante,
            msg.ubicacion_comensal,
            self.ip,
        );

        let logger = self.logger.clone();

        Box::pin(
            async move {
                for direccion in direcciones {
                    if let Ok(mut stream) = TcpStream::connect(direccion).await {
                        if let Ok(data) = serde_json::to_vec(&pedir_msg) {
                            let _ = stream.write_u8(PEDIR_CODE).await;
                            let _ = stream.write_u32(data.len() as u32).await;
                            let _ = stream.write_all(&data).await;

                            let _ = logger.info(
                                &format!("Pedir enviado a {}", direccion),
                                colored::Color::Cyan,
                            );
                            break;
                        } else {
                            println!("Error al serializar Pedir");
                        }
                    }
                }
            }
            .into_actor(self),
        )
    }
}

impl Handler<LlegoPedido> for NodoComensal {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: LlegoPedido, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            "Recibo LlegoPedido del NodoRestaurante",
            colored::Color::Cyan,
        );

        let comensal = self.comensal.clone();

        let _ = self
            .logger
            .info("Reenvío LlegoPedido al Comensal", colored::Color::Cyan);

        Box::pin(
            async move {
                comensal.do_send(msg);
            }
            .into_actor(self),
        )
    }
}
