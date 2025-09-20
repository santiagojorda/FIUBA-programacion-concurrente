use std::{
    net::{IpAddr, SocketAddrV4},
    time::Duration,
};

use super::nodo_repartidor::NodoRepartidor;
use crate::{
    estructuras_aux::{estado_pedido::EstadoPedido, pedido::Pedido, ubicacion::Ubicacion},
    logger::log::Logger,
    mensajes::{
        addr_nodo::AddrNodo, cobrar::Cobrar, info_pedido_en_curso::InfoPedidoEnCurso,
        llego_pedido::LlegoPedido, ofrecer_pedido::OfrecerPedido, pedido_tomado::PedidoTomado,
    },
};
use actix::prelude::*;
use actix::{Actor, Addr, Context};
use rand::Rng;

pub struct Repartidor {
    ubicacion: Ubicacion,
    pedido_actual: Option<Pedido>,
    restaurante_actual: Option<(u32, Ubicacion)>,
    dir_comensal_actual: Option<SocketAddrV4>,
    nodo_repartidor: Option<Addr<NodoRepartidor>>,
    logger: Logger,
}

impl Actor for Repartidor {
    type Context = Context<Self>;
}

impl Repartidor {
    pub fn new(ubicacion: &Ubicacion, logger: Logger) -> Self {
        Repartidor {
            ubicacion: *ubicacion,
            pedido_actual: None,
            restaurante_actual: None,
            dir_comensal_actual: None,
            nodo_repartidor: None,
            logger,
        }
    }

    fn calcular_tiempos(
        &self,
        pedido: &Pedido,
        ubicacion_restaurante: &Ubicacion,
    ) -> (Duration, Duration) {
        let ubicacion_inicial = self.ubicacion;
        let distancia_restaurante = ubicacion_inicial.distancia(ubicacion_restaurante);
        let distancia_restaurante_a_comensal =
            pedido.ubicacion_comensal().distancia(ubicacion_restaurante);
        let tiempo_hasta_restaurante =
            Duration::from_secs((0.5 + distancia_restaurante * 0.05) as u64);
        let tiempo_entrega =
            Duration::from_secs((0.5 + distancia_restaurante_a_comensal * 0.05) as u64);
        (tiempo_hasta_restaurante, tiempo_entrega)
    }

    async fn viajar_al_restaurante(
        nodo_repartidor: &Addr<NodoRepartidor>,
        pedido: Pedido,
        ubicacion_restaurante: Ubicacion,
        tiempo_hasta_restaurante: Duration,
        logger: Logger,
    ) {
        let _ = logger.info(
                    &format!(
                        "Comenzando viaje hasta el restaurante con ubicacion {:?} para entregar el pedido {}. Tiempo de viaje: {:?}",
                        ubicacion_restaurante,
                        pedido.id(),
                        tiempo_hasta_restaurante
                    ),
                    colored::Color::Cyan
                );
        actix_rt::time::sleep(tiempo_hasta_restaurante).await;
        nodo_repartidor.do_send(PedidoTomado::new(pedido.id(), pedido.id_restaurante()));
    }

    async fn entregar_al_comensal(
        nodo_repartidor: &Addr<NodoRepartidor>,
        logger: Logger,
        pedido: Pedido,
        dir_comensal: SocketAddrV4,
        tiempo_entrega: Duration,
        ubicacion_final: Ubicacion,
    ) {
        let _ = logger.info(
                    &format!(
                        "Comenzando viaje hasta el comensal con ubicacion {:?} para entregar el pedido {}. Tiempo de viaje: {:?}",
                        ubicacion_final,
                        pedido.id(),
                        tiempo_entrega
                    ),
                    colored::Color::Cyan
                );
        actix_rt::time::sleep(tiempo_entrega).await;
        let _ = logger.info(
            &format!(
                "Pedido {} entregado al comensal en {:?}.",
                pedido.id(),
                ubicacion_final
            ),
            colored::Color::Cyan,
        );
        nodo_repartidor.do_send(LlegoPedido::new(pedido.id(), dir_comensal.into()));
    }

    async fn cobrar_pedido(nodo_repartidor: &Addr<NodoRepartidor>, pedido: Pedido, logger: Logger) {
        let _ = logger.info(
            &format!("Pedido {} cobrado.", pedido.id()),
            colored::Color::Cyan,
        );
        nodo_repartidor.do_send(Cobrar::new(pedido.id()));
    }

    fn entregar_pedido(&mut self) -> ResponseActFuture<Self, bool> {
        let pedido = match self.pedido_actual.clone() {
            Some(p) => p,
            None => unreachable!(),
        };
        let (_, ubicacion_restaurante) = match &self.restaurante_actual {
            Some(t) => *t,
            None => unreachable!(),
        };
        let dir_comensal = match self.dir_comensal_actual {
            Some(d) => d,
            None => unreachable!(),
        };
        let nodo_repartidor = match self.nodo_repartidor.clone() {
            Some(n) => n,
            None => unreachable!(),
        };
        let (tiempo_hasta_restaurante, tiempo_entrega) =
            self.calcular_tiempos(&pedido, &ubicacion_restaurante);
        let ubicacion_final = *pedido.ubicacion_comensal();
        let logger = self.logger.clone();
        Box::pin(
            async move {
                Self::viajar_al_restaurante(
                    &nodo_repartidor,
                    pedido.clone(),
                    ubicacion_restaurante,
                    tiempo_hasta_restaurante,
                    logger.clone(),
                )
                .await;
                Self::entregar_al_comensal(
                    &nodo_repartidor,
                    logger.clone(),
                    pedido.clone(),
                    dir_comensal,
                    tiempo_entrega,
                    ubicacion_final,
                )
                .await;
                Self::cobrar_pedido(&nodo_repartidor, pedido, logger).await;
            }
            .into_actor(self)
            .map(move |_, actor, _| {
                actor.ubicacion = ubicacion_final;
                actor.pedido_actual = None;
                actor.restaurante_actual = None;
                actor.dir_comensal_actual = None;
                true
            }),
        )
    }

    fn aceptar_pedido(&mut self, msg: OfrecerPedido) {
        let _ = self.logger.info(
            &format!(
                "Repartidor aceptando pedido: {} (ubicación: {:?})",
                msg.pedido().id_pedido(),
                self.ubicacion
            ),
            colored::Color::Cyan,
        );
        let pedido = Pedido::new(
            msg.pedido().id_pedido(),
            EstadoPedido::Pendiente,
            0.0,
            msg.pedido().id_restaurante(),
            *msg.pedido().ubicacion_comensal(),
        );
        self.pedido_actual = Some(pedido);
        self.restaurante_actual = Some((
            msg.pedido().id_restaurante(),
            *msg.pedido().ubicacion_restaurante(),
        ));
        if let IpAddr::V4(ip) = msg.pedido().direccion_comensal().ip() {
            self.dir_comensal_actual = Some(SocketAddrV4::new(
                ip,
                msg.pedido().direccion_comensal().port(),
            ));
        } else {
            self.dir_comensal_actual = None;
        }
    }
}

impl Handler<OfrecerPedido> for Repartidor {
    type Result = ResponseActFuture<Self, bool>;

    fn handle(&mut self, msg: OfrecerPedido, _ctx: &mut Self::Context) -> Self::Result {
        let distancia = self
            .ubicacion
            .distancia(msg.pedido().ubicacion_restaurante());
        if self.pedido_actual.is_none() && distancia < 100.0 {
            let rng = rand::thread_rng().gen_range(0.0..1.0);
            if rng < 0.75 {
                self.aceptar_pedido(msg);
                return self.entregar_pedido();
            }
        }

        let _ = self.logger.warn(&format!(
            "Repartidor declinando pedido: {} (ubicación: {:?})",
            msg.pedido().id_pedido(),
            self.ubicacion
        ));

        Box::pin(async move { false }.into_actor(self))
    }
}

impl Handler<AddrNodo<NodoRepartidor>> for Repartidor {
    type Result = ();

    fn handle(&mut self, msg: AddrNodo<NodoRepartidor>, _ctx: &mut Self::Context) -> Self::Result {
        self.nodo_repartidor = Some(msg.get_addr().clone());
    }
}

impl Handler<InfoPedidoEnCurso> for Repartidor {
    type Result = ();

    fn handle(&mut self, msg: InfoPedidoEnCurso, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Repartidor recibiendo información del pedido en curso: {}",
                msg.pedido().id()
            ),
            colored::Color::Cyan,
        );
        self.dir_comensal_actual = Some(msg.dir_comensal());
        self.pedido_actual = Some(msg.pedido().clone());
        self.restaurante_actual = msg.info_restaurante().cloned();
    }
}
