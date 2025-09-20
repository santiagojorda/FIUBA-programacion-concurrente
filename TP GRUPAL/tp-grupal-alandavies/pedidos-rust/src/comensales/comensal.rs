use crate::{
    comensales::nodo_comensal::NodoComensal,
    estructuras_aux::{estado_pedido::EstadoPedido, pedido::Pedido, ubicacion::Ubicacion},
    logger::log::Logger,
    mensajes::{
        addr_nodo::AddrNodo, info_restaurantes::InfoRestaurantes, llego_pedido::LlegoPedido,
        pago_rechazado::PagoRechazado, pedir::Pedir, progreso_pedido::ProgresoPedido,
        ver_restaurantes_interno::VerRestaurantesInterno,
    },
};
use actix::{Actor, Addr, AsyncContext, Context, Handler, WrapFuture};
use rand::{thread_rng, Rng};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::time::sleep;

/// Representa un comensal que puede realizar pedidos a restaurantes.
///
/// Campos:
/// - `id`: Identificador único del comensal.
/// - `ubicacion`: Ubicación del comensal.
/// - `nodo_comensal`: Dirección del nodo comensal asociado.
/// - `pedido_realizado`: Pedido actual realizado por el comensal, si existe.
pub struct Comensal {
    id: u32,
    ubicacion: Ubicacion,
    nodo_comensal: Option<Addr<NodoComensal>>,
    pedido_realizado: Option<Pedido>,
    logger: Logger,
}

impl Actor for Comensal {
    type Context = Context<Self>;
}

impl Comensal {
    pub fn new(
        id: u32,
        ubicacion: Ubicacion,
        nodo_comensal: Option<Addr<NodoComensal>>,
        logger: Logger,
    ) -> Self {
        Comensal {
            id,
            ubicacion,
            nodo_comensal,
            pedido_realizado: None,
            logger,
        }
    }
}

impl Handler<InfoRestaurantes> for Comensal {
    type Result = ();

    /// Envía un mensaje al nodo comensal para solicitar información de restaurantes cercanos.
    fn handle(&mut self, msg: InfoRestaurantes, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            "Recibo InfoRestaurantes del nodo comensal",
            colored::Color::Cyan,
        );

        for (id_restaurante, ubicacion) in msg.info_restaurantes() {
            if self.pedido_realizado.is_none() && ubicacion.distancia(&self.ubicacion) < 100.0 {
                let mut rng = thread_rng();
                let pedido = Pedido::new(
                    rng.gen(),
                    EstadoPedido::Pendiente,
                    5000.0,
                    *id_restaurante,
                    self.ubicacion,
                );
                self.pedido_realizado = Some(pedido.clone());
                let mensaje = Pedir::new(
                    pedido.id(),
                    pedido.monto(),
                    pedido.id_restaurante(),
                    *pedido.ubicacion_comensal(),
                    SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 8080),
                );
                if let Some(ref nodo_comensal) = self.nodo_comensal {
                    nodo_comensal.do_send(mensaje);
                }
                let _ = self.logger.info(
                    &format!("Envio Pedir al restaurante {}", pedido.id_restaurante()),
                    colored::Color::Cyan,
                );
                break;
            }
        }
    }
}

impl Handler<PagoRechazado> for Comensal {
    type Result = ();

    /// Espera unos segundos y vuelve a solicitar la información de restaurantes cercanos.
    fn handle(&mut self, msg: PagoRechazado, ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!("Recibi PagoRechazado para el pedido {}", msg.id_pedido()),
            colored::Color::BrightYellow,
        );
        self.pedido_realizado = None;

        let nodo_comensal = self.nodo_comensal.clone();
        let id_comensal = self.id;
        let ubicacion_comensal = self.ubicacion;

        let fut = async move {
            sleep(Duration::from_secs(10)).await;
            if let Some(ref nodo_comensal) = nodo_comensal {
                nodo_comensal.do_send(VerRestaurantesInterno::new(id_comensal, ubicacion_comensal));
            }
        };

        let _ = self.logger.info(
            "Volviendo a solicitar información de restaurantes",
            colored::Color::Cyan,
        );

        ctx.spawn(fut.into_actor(self));
    }
}

impl Handler<ProgresoPedido> for Comensal {
    type Result = ();

    /// Solo lo recibe en caso de pedido cancelado o finalizado exitosamente.
    /// En caso de ser un pedido cancelado, descarta el pedido actual y vuelve a solicitar información de restaurantes cercanos para realizar un nuevo pedido.
    fn handle(&mut self, msg: ProgresoPedido, ctx: &mut Self::Context) -> Self::Result {
        if *msg.estado_pedido() == EstadoPedido::Cancelado {
            let _ = self.logger.error(&format!(
                "Recibi ProgresoPedido Cancelado para el pedido {}",
                msg.id_pedido()
            ));
            self.pedido_realizado = None;

            let nodo_comensal = self.nodo_comensal.clone();
            let id_comensal = self.id;
            let ubicacion_comensal = self.ubicacion;

            let fut = async move {
                sleep(Duration::from_secs(10)).await;
                if let Some(ref nodo_comensal) = nodo_comensal {
                    nodo_comensal
                        .do_send(VerRestaurantesInterno::new(id_comensal, ubicacion_comensal));
                }
            };
            let _ = self.logger.info(
                "Volviendo a solicitar información de restaurantes",
                colored::Color::Cyan,
            );

            ctx.spawn(fut.into_actor(self));
        }
    }
}

impl Handler<AddrNodo<NodoComensal>> for Comensal {
    type Result = ();

    /// Actualiza la dirección del nodo comensal asociado al comensal.
    fn handle(&mut self, msg: AddrNodo<NodoComensal>, _ctx: &mut Self::Context) -> Self::Result {
        self.nodo_comensal = Some(msg.get_addr().clone());
    }
}

impl Handler<LlegoPedido> for Comensal {
    type Result = ();

    /// Puede volver a hacer un nuevo pedido
    fn handle(&mut self, msg: LlegoPedido, ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!("Recibi LlegoPedido para el pedido {}", msg.id_pedido()),
            colored::Color::Green,
        );
        self.pedido_realizado = None;

        let nodo_comensal = self.nodo_comensal.clone();
        let id_comensal = self.id;
        let ubicacion_comensal = self.ubicacion;

        let fut = async move {
            sleep(Duration::from_secs(10)).await;
            if let Some(ref nodo_comensal) = nodo_comensal {
                nodo_comensal.do_send(VerRestaurantesInterno::new(id_comensal, ubicacion_comensal));
            }
        };
        let _ = self.logger.info(
            "Volviendo a solicitar información de restaurantes",
            colored::Color::Cyan,
        );

        ctx.spawn(fut.into_actor(self));
    }
}
