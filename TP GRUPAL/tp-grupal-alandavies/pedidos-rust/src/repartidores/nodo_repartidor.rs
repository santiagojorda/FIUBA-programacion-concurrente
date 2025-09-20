use super::repartidor::Repartidor;
use crate::estructuras_aux::estado_pedido::EstadoPedido;
use crate::estructuras_aux::pedido::Pedido;
use crate::logger::log::Logger;
use crate::mensajes::actualizar_siguiente::ActualizarSiguiente;
use crate::mensajes::cambiar_siguiente::CambiarSiguiente;
use crate::mensajes::cobrar::Cobrar;
use crate::mensajes::info_pedido_en_curso::InfoPedidoEnCurso;
use crate::mensajes::info_repartidor_pedido::InfoRepartidorPedido;
use crate::mensajes::keep_alive::KeepAlive;
use crate::mensajes::llego_pedido::LlegoPedido;
use crate::mensajes::mcode::{
    ACTUALIZAR_SIGUIENTE_CODE, CAMBIAR_SIGUIENTE_CODE, COBRAR_CODE, INFO_PEDIDO_EN_CURSO_CODE,
    INFO_REPARTIDOR_PEDIDO_CODE, KEEP_ALIVE_CODE, LLEGO_PEDIDO_CODE, OFRECER_PEDIDO_CODE,
    PEDIDOS_EN_CURSO_REPARTIDORES_CODE, PEDIDO_TOMADO_CODE, PROGRESO_PEDIDO_CODE,
    QUITAR_PEDIDO_DE_BUFFER_CODE, RING_ELECTION_CODE, SOLICITAR_PEDIDO_EN_CURSO_CODE,
};
use crate::mensajes::ofrecer_pedido::OfrecerPedido;
use crate::mensajes::pedido_listo::PedidoListo;
use crate::mensajes::pedido_tomado::PedidoTomado;
use crate::mensajes::pedidos_en_curso_repartidores::PedidosEnCursoRepartidores;
use crate::mensajes::progreso_pedido::ProgresoPedido;
use crate::mensajes::quitar_pedido_de_buffer::QuitarPedidoDeBuffer;
use crate::mensajes::ring_election::{RingElection, TIPO_ELECTION};
use crate::mensajes::solicitar_pedido_en_curso::SolicitarPedidoEnCurso;
use actix::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Serialize;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};

pub struct NodoRepartidor {
    repartidor: Addr<Repartidor>,
    pagos: SocketAddrV4,
    local: SocketAddrV4,
    lider: Option<SocketAddrV4>,
    siguiente: SocketAddrV4,
    anterior: SocketAddrV4,
    rango_nodos: (u16, u16),
    id_pedido_actual: Option<u32>,
    ip_restaurantes: Ipv4Addr,
    rango_restaurantes: (u16, u16),
    anillo_actual: Vec<u16>,
    pedidos_en_curso: Vec<InfoRepartidorPedido>,
    logger: Logger,
}

impl Actor for NodoRepartidor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let _ = self.logger.info(
            &format!(
                "Iniciado - Siguiente: {}, Anterior: {}",
                self.siguiente, self.anterior
            ),
            colored::Color::Cyan,
        );
        ctx.run_interval(std::time::Duration::from_secs(10), |actor, ctx| {
            actor.enviar_keep_alive(ctx);
        });
        ctx.run_interval(std::time::Duration::from_secs(20), |actor, ctx| {
            let lider = match actor.lider {
                Some(l) => l,
                None => {
                    let (puerto_local, siguiente, logger) =
                        (actor.local.port(), actor.siguiente, actor.logger.clone());
                    let fut = async move {
                        Self::comenzar_eleccion(puerto_local, siguiente, &logger).await;
                    }
                    .into_actor(actor);
                    ctx.spawn(fut);
                    return;
                }
            };
            if lider != actor.local {
                return;
            }
            actor
                .pedidos_a_cancelar()
                .into_iter()
                .for_each(|(pedido_id, dir_comensal)| {
                    actor.cancelar_pedido(pedido_id, dir_comensal, ctx);
                })
        });
    }
}

impl NodoRepartidor {
    pub fn new(
        repartidor: Addr<Repartidor>,
        pagos: SocketAddrV4,
        local: SocketAddrV4,
        rango_nodos: (u16, u16),
        ip_restaurantes: Ipv4Addr,
        rango_restaurantes: (u16, u16),
        logger: Logger,
    ) -> Self {
        let siguiente = if local.port() == rango_nodos.1 {
            SocketAddrV4::new(*local.ip(), rango_nodos.0)
        } else {
            SocketAddrV4::new(*local.ip(), local.port() + 1)
        };
        let anterior = if local.port() == rango_nodos.0 {
            SocketAddrV4::new(*local.ip(), rango_nodos.1)
        } else {
            SocketAddrV4::new(*local.ip(), local.port() - 1)
        };

        let _ = logger.info(
            &format!(
                "Nodo creado - Siguiente: {}, Anterior: {}",
                siguiente, anterior
            ),
            colored::Color::Cyan,
        );

        NodoRepartidor {
            repartidor,
            pagos,
            local,
            lider: None,
            siguiente,
            anterior,
            rango_nodos,
            id_pedido_actual: None,
            ip_restaurantes,
            rango_restaurantes,
            anillo_actual: Vec::new(),
            pedidos_en_curso: Vec::new(),
            logger,
        }
    }

    pub async fn enviar_mensaje<M>(
        mcode: u8,
        msg: &M,
        receptor: SocketAddrV4,
    ) -> Result<(), std::io::Error>
    where
        M: Message + Send + 'static + Serialize,
    {
        let mut stream = TcpStream::connect(receptor).await?;
        let data = serde_json::to_vec(msg)?;
        stream.write_u8(mcode).await?;
        stream.write_u32(data.len() as u32).await?;
        stream.write_all(&data).await?;
        Ok(())
    }

    pub async fn enviar_mensaje_udp<M>(
        mcode: u8,
        msg: &M,
        receptor: SocketAddrV4,
    ) -> Result<(), std::io::Error>
    where
        M: Message + Send + 'static + Serialize,
    {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let serialized_msg = serde_json::to_vec(msg)?;
        let mut data: Vec<u8> = Vec::new();
        data.push(mcode);
        data.extend(serialized_msg.len().to_be_bytes());
        data.extend(serialized_msg);
        socket.send_to(&data, receptor).await?;
        Ok(())
    }

    async fn ofrecer_pedido_al_siguiente(
        msg: &OfrecerPedido,
        siguiente: SocketAddrV4,
        local: SocketAddrV4,
    ) {
        if Self::enviar_mensaje(OFRECER_PEDIDO_CODE, msg, siguiente)
            .await
            .is_err()
        {
            let _ = Self::enviar_mensaje(
                ACTUALIZAR_SIGUIENTE_CODE,
                &ActualizarSiguiente::new(),
                local,
            )
            .await;
            actix_rt::time::sleep(std::time::Duration::from_secs(10)).await;
            let _ = Self::enviar_mensaje(OFRECER_PEDIDO_CODE, msg, siguiente).await;
        }
    }

    fn pedidos_a_cancelar(&self) -> Vec<(u32, SocketAddrV4)> {
        self.pedidos_en_curso
            .iter()
            .filter(|pedido| {
                (pedido.timestamp() + 120) * 1000 >= chrono::Utc::now().timestamp_millis()
                    && !self.anillo_actual.contains(&pedido.dir_repartidor().port())
            })
            .map(|pedido| (pedido.id_pedido(), pedido.dir_comensal()))
            .collect()
    }

    fn ring_election_handle_tipo_election(&mut self, msg: &mut RingElection) {
        if msg.puerto_original() == self.local.port() {
            msg.cambiar_a_coordinator();
            self.lider = Some(SocketAddrV4::new(
                *self.local.ip(),
                msg.puerto_maximo().unwrap(),
            ));
            self.anillo_actual = msg.puertos.clone();
            let _ = self.logger.info(
                &format!(
                    "Nuevo líder: {:?}, Anillo: {:?}",
                    self.lider, self.anillo_actual
                ),
                colored::Color::White,
            );
        } else {
            msg.agregar_puerto(self.local.port());
        }
    }

    fn ring_election_handle_tipo_coordinator(&mut self, msg: &mut RingElection) -> bool {
        let nuevo_lider = SocketAddrV4::new(*self.local.ip(), msg.puerto_maximo().unwrap());
        let _ = self.logger.info(
            &format!("Estableciendo líder: {}", nuevo_lider),
            colored::Color::Cyan,
        );
        self.lider = Some(nuevo_lider);

        self.anillo_actual = msg.puertos.clone();
        let _ = self.logger.info(
            &format!("Anillo actualizado: {:?}", self.anillo_actual),
            colored::Color::Cyan,
        );
        if msg.puerto_original() == self.local.port() {
            let _ = self
                .logger
                .info("Elección completada, no reenvío", colored::Color::Cyan);
            return false;
        }
        true
    }

    async fn enviar_pedidos_en_curso(
        local: &SocketAddrV4,
        pedidos_en_curso: Vec<InfoRepartidorPedido>,
        logger: &Logger,
        puerto_maximo: u16,
    ) {
        let _ = logger.info(
            &format!(
                "Era líder, enviando pedidos en curso ({}) al nuevo líder",
                pedidos_en_curso.len()
            ),
            colored::Color::White,
        );
        let _ = Self::enviar_mensaje(
            PEDIDOS_EN_CURSO_REPARTIDORES_CODE,
            &PedidosEnCursoRepartidores::new(pedidos_en_curso),
            SocketAddrV4::new(*local.ip(), puerto_maximo),
        )
        .await;
    }

    fn comenzar_eleccion_y_reenviar(
        &self,
        receptor: SocketAddrV4,
        mcode: u8,
        msg: impl Serialize + Send + 'static + Message,
    ) -> ResponseActFuture<Self, ()> {
        let logger = self.logger.clone();
        let local = self.local.port();
        let siguiente = self.siguiente;
        Box::pin(
            async move {
                Self::comenzar_eleccion(local, siguiente, &logger).await;
                actix_rt::time::sleep(std::time::Duration::from_secs(10)).await;
                let _ = Self::enviar_mensaje(mcode, &msg, receptor).await;
            }
            .into_actor(self),
        )
    }

    async fn enviar_mensaje_con_fallback(
        msg: impl Serialize + Send + 'static + Message,
        mcode: u8,
        receptor: SocketAddrV4,
        siguiente: SocketAddrV4,
        local: SocketAddrV4,
        lider: SocketAddrV4,
        logger: &Logger,
    ) {
        if Self::enviar_mensaje(mcode, &msg, receptor).await.is_err() {
            if receptor == siguiente {
                let _ = logger.warn(&format!(
                    "Error al enviar mensaje a {}, actualizando siguiente",
                    receptor
                ));
                let _ = Self::enviar_mensaje(
                    ACTUALIZAR_SIGUIENTE_CODE,
                    &ActualizarSiguiente::new(),
                    local,
                )
                .await;
            }
            if receptor == lider {
                let _ = logger.warn(&format!(
                    "Error al enviar mensaje a líder {}, iniciando elección",
                    receptor
                ));
                Self::comenzar_eleccion(local.port(), siguiente, logger).await;
            }
            actix_rt::time::sleep(std::time::Duration::from_secs(10)).await;
            let _ = Self::enviar_mensaje(mcode, &msg, receptor).await;
        };
    }

    async fn buscar_siguiente(
        rango: (u16, u16),
        local_port: u16,
        ip: Ipv4Addr,
        siguiente_actual: u16,
    ) -> SocketAddrV4 {
        let range_size = (rango.1 - rango.0 + 1) as usize;
        let start_offset = (siguiente_actual - rango.0) as usize;

        for i in 0..range_size {
            let port_offset = (start_offset + i) % range_size;
            let siguiente_port = rango.0 + port_offset as u16;

            if siguiente_port == local_port {
                continue;
            }

            let addr = SocketAddrV4::new(ip, siguiente_port);
            if TcpStream::connect(addr).await.is_ok() {
                println!(
                    "[NodoRepartidor {}] Nuevo siguiente encontrado: {}",
                    local_port, addr
                );
                let _ = std::io::stdout().flush();

                return addr;
            }
        }

        SocketAddrV4::new(
            ip,
            if local_port < rango.1 {
                local_port + 1
            } else {
                rango.0
            },
        )
    }

    async fn comenzar_eleccion(puerto_local: u16, siguiente: SocketAddrV4, logger: &Logger) {
        let _ = logger.warn("No hay líder, iniciando elección");
        let _ = Self::enviar_mensaje(
            RING_ELECTION_CODE,
            &RingElection::new(puerto_local),
            siguiente,
        )
        .await;
    }

    fn enviar_keep_alive(&self, ctx: &mut <NodoRepartidor as actix::Actor>::Context) {
        let siguiente = self.siguiente;
        let local = self.local;
        let fut = async move {
            Self::enviar_mensaje(KEEP_ALIVE_CODE, &KeepAlive::new(local), siguiente).await
        }
        .into_actor(self)
        .map(|res, actor, ctx| {
            if res.is_err() {
                let _ = actor
                    .logger
                    .error("Error al enviar KeepAlive, actualizando siguiente");
                ctx.address().do_send(ActualizarSiguiente);
            }
        });
        ctx.spawn(fut);
    }

    fn cancelar_pedido(
        &mut self,
        id_pedido: u32,
        dir_comensal: SocketAddrV4,
        ctx: &mut <NodoRepartidor as actix::Actor>::Context,
    ) {
        let logger = self.logger.clone();
        let fut = async move {
            if Self::enviar_mensaje_udp(
                PROGRESO_PEDIDO_CODE,
                &ProgresoPedido::new(EstadoPedido::Cancelado, id_pedido),
                dir_comensal,
            )
            .await
            .is_ok()
            {
                let _ = logger.info(
                    &format!(
                        "Aviso de pedido ({}) cancelado enviado a {}",
                        id_pedido, dir_comensal
                    ),
                    colored::Color::Blue,
                );
            } else {
                let _ = logger.error(&format!(
                    "Error al enviar cancelación del pedido {} a {}",
                    id_pedido, dir_comensal
                ));
            }
        }
        .into_actor(self)
        .map(move |_, actor, _| {
            actor
                .pedidos_en_curso
                .retain(|p| p.id_pedido() != id_pedido);
        });
        ctx.spawn(fut);
    }

    fn ofrecer_pedido_a_restaurante(
        &mut self,
        ofrecimiento: OfrecerPedido,
    ) -> ResponseActFuture<Self, bool> {
        let lider = match self.lider {
            Some(l) => l,
            None => {
                return Box::pin(
                    self.comenzar_eleccion_y_reenviar(
                        self.local,
                        OFRECER_PEDIDO_CODE,
                        ofrecimiento,
                    )
                    .map(|_, _, _| false),
                )
            }
        };
        let (siguiente, local, repartidor, msg_clone, ocupado, logger) = (
            self.siguiente,
            self.local,
            self.repartidor.clone(),
            ofrecimiento.clone(),
            self.id_pedido_actual.is_some(),
            self.logger.clone(),
        );
        self.id_pedido_actual = Some(ofrecimiento.pedido().id_pedido());
        let fut = async move {
            if ocupado || !repartidor.send(msg_clone.clone()).await.unwrap_or(false) {
                Self::ofrecer_pedido_al_siguiente(&msg_clone, siguiente, local).await;
                return false;
            }
            if let IpAddr::V4(ip) = msg_clone.pedido().direccion_comensal().ip() {
                Self::enviar_mensaje_con_fallback(
                    InfoPedidoEnCurso::new(
                        SocketAddrV4::new(ip, msg_clone.pedido().direccion_comensal().port()),
                        Pedido::new(
                            msg_clone.pedido().id_pedido(),
                            EstadoPedido::Listo,
                            0.0,
                            msg_clone.pedido().id_restaurante(),
                            *msg_clone.pedido().ubicacion_comensal(),
                        ),
                        Some((
                            msg_clone.pedido().id_restaurante(),
                            *msg_clone.pedido().ubicacion_restaurante(),
                        )),
                    ),
                    INFO_PEDIDO_EN_CURSO_CODE,
                    lider,
                    siguiente,
                    local,
                    lider,
                    &logger,
                )
                .await;
            }
            true
        }
        .into_actor(self)
        .map(move |res, actor, _| {
            if !res {
                actor.id_pedido_actual = None;
                let _ = actor.logger.warn(&format!(
                    "Declinado pedido {} (ocupado o repartidor no lo acepta)",
                    ofrecimiento.pedido().id_pedido()
                ));
            }
            res
        });

        Box::pin(fut)
    }
}

impl Handler<OfrecerPedido> for NodoRepartidor {
    type Result = ResponseActFuture<Self, bool>;

    fn handle(&mut self, msg: OfrecerPedido, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Recibido OfrecerPedido para pedido {}",
                msg.pedido().id_pedido()
            ),
            colored::Color::Cyan,
        );
        self.ofrecer_pedido_a_restaurante(msg)
    }
}

impl Handler<RingElection> for NodoRepartidor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: RingElection, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!("Recibido RingElection: {:?}", msg),
            colored::Color::Cyan,
        );
        let (mut msg, mut seguir_enviando, siguiente, pedidos_en_curso, local) = (
            msg.clone(),
            true,
            self.siguiente,
            self.pedidos_en_curso.clone(),
            self.local,
        );
        let era_lider = match self.lider {
            Some(lider) => lider == self.local,
            None => false,
        };
        if msg.tipo() == TIPO_ELECTION {
            self.ring_election_handle_tipo_election(&mut msg);
        } else {
            seguir_enviando = self.ring_election_handle_tipo_coordinator(&mut msg);
        }
        let logger = self.logger.clone();
        let fut = async move {
            if era_lider {
                if let Some(puerto_maximo) = msg.puerto_maximo() {
                    Self::enviar_pedidos_en_curso(&local, pedidos_en_curso, &logger, puerto_maximo)
                        .await;
                }
            }
            if seguir_enviando {
                let _ = Self::enviar_mensaje(RING_ELECTION_CODE, &msg, siguiente).await;
            }
        }
        .into_actor(self)
        .map(move |_, actor, _| {
            actor.pedidos_en_curso.clear();
        });
        Box::pin(fut)
    }
}

impl Handler<PedidoListo> for NodoRepartidor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: PedidoListo, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!("Recibido PedidoListo para pedido {}", msg.id_pedido()),
            colored::Color::Cyan,
        );
        Box::pin(
            self.ofrecer_pedido_a_restaurante(OfrecerPedido::new(msg))
                .map(|_, _, _| ()),
        )
    }
}

impl Handler<PedidoTomado> for NodoRepartidor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: PedidoTomado, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!("Recibido PedidoTomado para pedido {}", msg.id_pedido()),
            colored::Color::Cyan,
        );
        let mut puertos: Vec<u16> =
            (self.rango_restaurantes.0..=self.rango_restaurantes.1).collect();
        puertos.shuffle(&mut thread_rng());
        let ip_restaurantes = self.ip_restaurantes;
        let logger = self.logger.clone();
        let fut = async move {
            for puerto in puertos {
                let direccion_restaurante = SocketAddrV4::new(ip_restaurantes, puerto);
                if Self::enviar_mensaje(PEDIDO_TOMADO_CODE, &msg, direccion_restaurante)
                    .await
                    .is_ok()
                {
                    let _ = logger.info(
                        &format!(
                            "PedidoTomado {:?} enviado a restaurante {}",
                            msg, direccion_restaurante
                        ),
                        colored::Color::White,
                    );
                    break;
                } else {
                    let _ = logger.warn(&format!(
                        "No se pudo enviar PedidoTomado {:?} a {}, intentando siguiente",
                        msg, direccion_restaurante
                    ));
                }
            }
        }
        .into_actor(self);
        Box::pin(fut)
    }
}

impl Handler<Cobrar> for NodoRepartidor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: Cobrar, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!("Recibido Cobrar para pedido {}", msg.id_pedido()),
            colored::Color::Cyan,
        );
        let pagos = self.pagos;
        let siguiente = self.siguiente;
        let local = self.local;
        let logger = self.logger.clone();
        let lider = match self.lider {
            Some(l) => l,
            None => {
                return self.comenzar_eleccion_y_reenviar(local, COBRAR_CODE, msg);
            }
        };
        let id_pedido_actual = self.id_pedido_actual;
        let fut = async move {
            if Some(msg.id_pedido()) == id_pedido_actual {
                let _ = logger.info(
                    &format!(
                        "Enviando cobro a pagos {} para pedido {}",
                        pagos, msg.id_pedido
                    ),
                    colored::Color::Cyan,
                );
                let _ = Self::enviar_mensaje(COBRAR_CODE, &msg, pagos).await;
            }
            if !lider.eq(&local) {
                Self::enviar_mensaje_con_fallback(
                    QuitarPedidoDeBuffer::new(msg.id_pedido()),
                    QUITAR_PEDIDO_DE_BUFFER_CODE,
                    lider,
                    siguiente,
                    local,
                    lider,
                    &logger,
                )
                .await;
            }
        }
        .into_actor(self);
        Box::pin(fut)
    }
}

impl Handler<LlegoPedido> for NodoRepartidor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: LlegoPedido, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!("Recibido LlegoPedido para pedido {}", msg.id_pedido(),),
            colored::Color::Cyan,
        );
        let logger = self.logger.clone();
        let fut = async move {
            if let SocketAddr::V4(dir_comensal) = msg.direccion_comensal() {
                if Self::enviar_mensaje_udp(LLEGO_PEDIDO_CODE, &msg, *dir_comensal)
                    .await
                    .is_ok()
                {
                    let _ = logger.info(
                        &format!(
                            "Notificación LlegoPedido enviada a {}",
                            msg.direccion_comensal()
                        ),
                        colored::Color::White,
                    );
                } else {
                    let _ = logger.error(&format!(
                        "Error al enviar notificación LlegoPedido a {}",
                        msg.direccion_comensal()
                    ));
                };
            }
        }
        .into_actor(self)
        .map(move |_, actor, _| {
            actor.id_pedido_actual = None;
        });
        Box::pin(fut)
    }
}

impl Handler<ActualizarSiguiente> for NodoRepartidor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: ActualizarSiguiente, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Recibido ActualizarSiguiente, actualizando siguiente desde {}",
                self.siguiente
            ),
            colored::Color::Cyan,
        );
        let ip = *self.local.ip();
        let rango = self.rango_nodos;
        let local_port = self.local.port();
        let actual_port = self.siguiente.port();

        let fut = async move { Self::buscar_siguiente(rango, local_port, ip, actual_port).await }
            .into_actor(self)
            .map(|nuevo_addr, actor, _ctx| {
                let _ = actor.logger.info(
                    &format!(
                        "Siguiente actualizado de {} a {}",
                        actor.siguiente, nuevo_addr
                    ),
                    colored::Color::White,
                );
                actor.siguiente = nuevo_addr;
            });

        Box::pin(fut)
    }
}

impl Handler<KeepAlive> for NodoRepartidor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: KeepAlive, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!("Recibido KeepAlive de {}", msg.dir_emisor()),
            colored::Color::Cyan,
        );
        let (anterior, siguiente, puerto_local, dir_emisor, logger, local) = (
            self.anterior,
            self.siguiente,
            self.local.port(),
            msg.dir_emisor(),
            self.logger.clone(),
            self.local,
        );
        let lider = match self.lider {
            Some(l) => l,
            None => {
                return Box::pin(
                    async move {
                        Self::comenzar_eleccion(puerto_local, siguiente, &logger).await;
                    }
                    .into_actor(self),
                );
            }
        };
        if anterior.port() == dir_emisor.port() {
            return Box::pin(async move {}.into_actor(self));
        }
        let fut = async move {
            let _ = Self::enviar_mensaje(
                CAMBIAR_SIGUIENTE_CODE,
                &CambiarSiguiente::new(dir_emisor),
                anterior,
            )
            .await;
            Self::enviar_mensaje_con_fallback(
                SolicitarPedidoEnCurso::new(dir_emisor),
                SOLICITAR_PEDIDO_EN_CURSO_CODE,
                dir_emisor,
                siguiente,
                local,
                lider,
                &logger,
            )
            .await;
        }
        .into_actor(self)
        .map(move |_, actor, _| {
            actor.anterior = msg.dir_emisor();
        });
        Box::pin(fut)
    }
}

impl Handler<CambiarSiguiente> for NodoRepartidor {
    type Result = ();

    fn handle(&mut self, msg: CambiarSiguiente, _ctx: &mut Self::Context) -> Self::Result {
        println!(
            "[NodoRepartidor {}] Recibido CambiarSiguiente: {}",
            self.local.port(),
            msg.siguiente()
        );
        println!(
            "[NodoRepartidor {}] Siguiente cambiado de {} a {}",
            self.local.port(),
            self.siguiente,
            msg.siguiente()
        );
        self.siguiente = *msg.siguiente();
    }
}

impl Handler<InfoRepartidorPedido> for NodoRepartidor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: InfoRepartidorPedido, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Recibido InfoRepartidorPedido para pedido {}",
                msg.id_pedido()
            ),
            colored::Color::Cyan,
        );
        let siguiente = self.siguiente;
        let local = self.local;
        let lider = match self.lider {
            Some(l) => l,
            None => {
                return self.comenzar_eleccion_y_reenviar(local, INFO_REPARTIDOR_PEDIDO_CODE, msg);
            }
        };
        let logger = self.logger.clone();
        if self.local == lider {
            let _ = self.logger.info(
                &format!("Soy líder, agregando pedido {} a la lista", msg.id_pedido()),
                colored::Color::Blue,
            );
            self.pedidos_en_curso.push(msg);
            Box::pin(async move {}.into_actor(self))
        } else {
            let fut = async move {
                Self::enviar_mensaje_con_fallback(
                    msg,
                    INFO_REPARTIDOR_PEDIDO_CODE,
                    lider,
                    siguiente,
                    local,
                    lider,
                    &logger,
                )
                .await;
            }
            .into_actor(self);
            Box::pin(fut)
        }
    }
}

impl Handler<InfoPedidoEnCurso> for NodoRepartidor {
    type Result = ();

    fn handle(&mut self, msg: InfoPedidoEnCurso, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Recibido InfoPedidoEnCurso para pedido {}",
                msg.pedido().id(),
            ),
            colored::Color::Cyan,
        );
        self.id_pedido_actual = Some(msg.pedido().id());
        self.repartidor.do_send(msg);
    }
}

impl Handler<SolicitarPedidoEnCurso> for NodoRepartidor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: SolicitarPedidoEnCurso, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!("Recibido SolicitarPedidoEnCurso de {}", msg.dir_emisor()),
            colored::Color::Cyan,
        );
        if let Some(pedido) = self
            .pedidos_en_curso
            .iter()
            .find(|p| p.dir_repartidor() == msg.dir_emisor())
        {
            let _ = self.logger.info(
                &format!(
                    "Pedido en curso con id {} encontrado para {}",
                    pedido.id_pedido(),
                    msg.dir_emisor(),
                ),
                colored::Color::Blue,
            );
            let pedido = pedido.clone();
            let fut = async move {
                let _ = Self::enviar_mensaje(
                    INFO_PEDIDO_EN_CURSO_CODE,
                    &InfoPedidoEnCurso::new(
                        pedido.dir_comensal(),
                        pedido.pedido().clone(),
                        pedido.info_restaurante().cloned(),
                    ),
                    msg.dir_emisor(),
                )
                .await;
            }
            .into_actor(self);
            return Box::pin(fut);
        }
        let _ = self.logger.warn(&format!(
            "No se encontró pedido en curso para {}",
            msg.dir_emisor()
        ));
        Box::pin(async move {}.into_actor(self))
    }
}

impl Handler<PedidosEnCursoRepartidores> for NodoRepartidor {
    type Result = ();

    fn handle(
        &mut self,
        msg: PedidosEnCursoRepartidores,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Recibido PedidosEnCursoRepartidores con {} pedidos",
                msg.pedidos().len()
            ),
            colored::Color::Cyan,
        );
        self.pedidos_en_curso = msg.pedidos().clone();
    }
}

impl Handler<QuitarPedidoDeBuffer> for NodoRepartidor {
    type Result = ();

    fn handle(&mut self, msg: QuitarPedidoDeBuffer, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Recibido QuitarPedidoDeBuffer para pedido {}",
                msg.id_pedido()
            ),
            colored::Color::Cyan,
        );
        self.pedidos_en_curso
            .retain(|pedido| pedido.id_pedido() != msg.id_pedido());
    }
}
