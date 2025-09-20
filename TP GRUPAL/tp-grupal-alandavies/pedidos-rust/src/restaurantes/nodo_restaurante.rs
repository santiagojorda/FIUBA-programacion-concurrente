use crate::estructuras_aux::estado_pedido::EstadoPedido;
use crate::estructuras_aux::ubicacion::Ubicacion;
use crate::logger::log::Logger;
use crate::mensajes::actualizar_siguiente::ActualizarSiguiente;
use crate::mensajes::cambiar_siguiente::CambiarSiguiente;
use crate::mensajes::info_lider::InfoLider;
use crate::mensajes::info_restaurantes::InfoRestaurantes;
use crate::mensajes::info_restaurantes_interno::InfoRestaurantesInterno;
use crate::mensajes::keep_alive::KeepAlive;
use crate::mensajes::mcode::{
    AGREGAR_PEDIDO_EN_CURSO_CODE, CAMBIAR_SIGUIENTE_CODE, ELIMINAR_PEDIDO_EN_CURSO_CODE,
    INFO_LIDER_CODE, INFO_RESTAURANTES_CODE, INFO_RESTAURANTES_INTERNO_CODE, KEEP_ALIVE_CODE,
    PEDIDO_LISTO_CODE, PEDIDO_TOMADO_CODE, PEDIR_CODE, PROGRESO_PEDIDO_CODE, RING_ELECTION_CODE,
    VER_RESTAURANTES_CODE,
};
use crate::mensajes::obtener_ubicacion::ObtenerUbicacion;
use crate::mensajes::pedido_listo::PedidoListo;
use crate::mensajes::pedido_tomado::PedidoTomado;
use crate::mensajes::ring_election::{RingElection, TIPO_ELECTION};
use crate::mensajes::{
    pedir::Pedir, progreso_pedido::ProgresoPedido, ver_restaurantes::VerRestaurantes,
};
use crate::restaurantes::restaurante::Restaurante;
use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, Message, ResponseActFuture,
    WrapFuture,
};

use crate::mensajes::agrega_pedido_en_curso::AgregarPedidoEnCurso;
use crate::mensajes::eliminar_peido_en_curso::EliminarPedidoEnCurso;
use crate::mensajes::info_restaurante_pedido::InfoRestaurantePedido;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Serialize;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};

const MAX_DISTANCIA_RESTAURANTES: f32 = 100.0;

/// Representa un nodo que maneja la lógica de recepción del pedido de un comensal,
/// la preparación de este y su pasaje a repartidores.
///
/// # Campos
/// - `restaurante`: Tupla con la dirección del actor Restaurante y su ID
/// - `pedidos`: Mapa de pedidos en proceso (ID -> dirección del comensal y ubicación)
/// - `restaurantes`: Mapa de restaurantes conocidos (ID -> dirección)
/// - `ip_repartidores`: Dirección IP de los repartidores
/// - `rango_repartidores`: Rango de puertos para repartidores
/// - `local`: Dirección local del nodo
/// - `lider`: Dirección del nodo líder (None si no hay líder)
/// - `siguiente`: Dirección del siguiente nodo en el anillo
/// - `anterior`: Dirección del nodo anterior en el anillo
/// - `rango_nodos`: Rango de puertos para nodos de restaurante
#[derive(Debug)]
pub struct NodoRestaurante {
    restaurante: (Addr<Restaurante<NodoRestaurante>>, u32),
    pedidos: HashMap<u32, (SocketAddrV4, Ubicacion)>,
    pedidos_en_curso: Vec<InfoRestaurantePedido>,
    restaurantes: HashMap<u32, SocketAddrV4>,
    ip_repartidores: Ipv4Addr,
    rango_repartidores: (u16, u16),
    local: SocketAddrV4,
    lider: Option<SocketAddrV4>,
    siguiente: SocketAddrV4,
    anterior: SocketAddrV4,
    rango_nodos: (u16, u16),
    logger: Logger,
}

impl Actor for NodoRestaurante {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.print_cyan("Actor iniciado");

        ctx.run_interval(std::time::Duration::from_secs(2), |actor, ctx| {
            let siguiente = actor.siguiente;
            let local = actor.local;
            let logger = actor.logger.clone();
            let lider = actor.lider;

            let fut = async move {
                match TcpStream::connect(siguiente).await {
                    Ok(mut s) => {
                        if let Ok(serialized_msg) = serde_json::to_vec(&KeepAlive::new(local)) {
                            let _ = s.write_u8(KEEP_ALIVE_CODE).await;
                            let _ = s.write_u32(serialized_msg.len() as u32).await;
                            let _ = s.write_all(&serialized_msg).await;
                            Ok(())
                        } else {
                            Err(())
                        }
                    }
                    Err(_) => {
                        let _ = logger.error(&format!(
                            "No se pudo conectar a {}, actualizando siguiente",
                            siguiente
                        ));
                        if let Some(lider) = lider {
                            let _ = logger.error("No se pudo conectar al siguiente, enviando mensaje de eliminación de pedido en curso");
                            Self::enviar_mensaje(
                                ELIMINAR_PEDIDO_EN_CURSO_CODE,
                                &EliminarPedidoEnCurso::new(0, siguiente.port() as u32),
                                lider,
                            )
                            .await;
                        }
                        Err(())
                    }
                }
            }
            .into_actor(actor)
            .map(|res, actor, ctx| {
                if res.is_err() {
                    actor.print_error("Error al enviar KeepAlive, actualizando siguiente");

                    ctx.address().do_send(ActualizarSiguiente);
                }
            });
            ctx.spawn(fut);
        });
    }
}

impl NodoRestaurante {
    pub fn new(
        restaurante: (Addr<Restaurante<NodoRestaurante>>, u32),
        local: SocketAddrV4,
        rango_nodos: (u16, u16),
        ip_repartidores: Ipv4Addr,
        rango_repartidores: (u16, u16),
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
        let mut restaurantes = HashMap::new();

        for port in rango_nodos.0..=rango_nodos.1 {
            restaurantes.insert(port as u32, SocketAddrV4::new(*local.ip(), port));
        }
        NodoRestaurante {
            restaurante,
            pedidos: HashMap::new(),
            pedidos_en_curso: Vec::new(),
            restaurantes,
            ip_repartidores,
            rango_repartidores,
            local,
            lider: None,
            siguiente,
            anterior,
            rango_nodos,
            logger,
        }
    }

    fn print_cyan(&self, msg: &str) {
        let _ = self.logger.info(msg, colored::Color::Cyan);
    }

    fn print_white(&self, msg: &str) {
        let _ = self.logger.info(msg, colored::Color::White);
    }

    fn print_blue(&self, msg: &str) {
        let _ = self.logger.info(msg, colored::Color::Blue);
    }

    fn print_error(&self, msg: &str) {
        let _ = self.logger.error(msg);
    }

    /// Envía un mensaje serializado a otro nodo a través de TCP.
    pub async fn enviar_mensaje<M>(mcode: u8, msg: &M, receptor: SocketAddrV4)
    where
        M: Message + Send + 'static + Serialize,
    {
        if let Ok(mut stream) = TcpStream::connect(receptor).await {
            if let Ok(data) = serde_json::to_vec(msg) {
                let _ = stream.write_u8(mcode).await;
                let _ = stream.write_u32(data.len() as u32).await;
                let _ = stream.write_all(&data).await;
            }
        }
    }

    fn iniciar_votacion_nuevo_lider(&mut self, ctx: &mut Context<Self>) {
        let siguiente = self.siguiente;
        let puerto_local = self.local.port();
        let fut = async move {
            Self::enviar_mensaje(
                RING_ELECTION_CODE,
                &RingElection::new(puerto_local),
                siguiente,
            )
            .await;
        }
        .into_actor(self);

        ctx.spawn(fut);
    }
}

impl Handler<ProgresoPedido> for NodoRestaurante {
    type Result = ResponseActFuture<Self, ()>;

    /// Maneja actualizaciones sobre el estado de los pedidos (Pendiente, Preparado, etc.)
    /// y notifica a los comensales correspondientes.
    /// Además, también le avisa a los repartidores que esta listo para ser tomado.
    fn handle(&mut self, msg: ProgresoPedido, _ctx: &mut Self::Context) -> Self::Result {
        self.print_white(&format!(
            "Recibido ProgresoPedido con estado {:?}",
            msg.estado_pedido()
        ));
        if let EstadoPedido::Listo = msg.estado_pedido() {
            let restaurante_addr = self.restaurante.0.clone();
            let restaurante_id = self.restaurante.1;
            let id_pedido = msg.id_pedido();
            let ip_repartidor = self.ip_repartidores;
            let logger = self.logger.clone();
            let mut puertos: Vec<u16> =
                (self.rango_repartidores.0..=self.rango_repartidores.1).collect();
            puertos.shuffle(&mut thread_rng());
            if let Some(pedido) = self.pedidos.get(&id_pedido).cloned() {
                self.print_cyan(&format!(
                    "Enviando PedidoListo a Repartidores pedido: {}",
                    msg.id_pedido()
                ));
                return Box::pin(
                    async move {
                        if let Ok(ubicacion) = restaurante_addr.send(ObtenerUbicacion::new()).await
                        {
                            let msg_pedido_listo = PedidoListo::new(
                                id_pedido,
                                restaurante_id,
                                ubicacion,
                                pedido.1,
                                std::net::SocketAddr::V4(pedido.0),
                            );
                            for puerto_repartidor in puertos {
                                match TcpStream::connect(SocketAddrV4::new(
                                    ip_repartidor,
                                    puerto_repartidor,
                                ))
                                .await
                                {
                                    Ok(mut stream) => {
                                        if let Ok(data) = serde_json::to_vec(&msg_pedido_listo) {
                                            let _ = stream.write_u8(PEDIDO_LISTO_CODE).await;
                                            let _ = stream.write_u32(data.len() as u32).await;
                                            let _ = stream.write_all(&data).await;
                                        } else {
                                            let _ = logger.error("Error al enviar PedidoListo");
                                        }
                                        break;
                                    }
                                    _ => {
                                        let _ = logger.error(&format!(
                                            "Repartidor {:?} no conectado",
                                            ip_repartidor
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    .into_actor(self),
                );
            } else {
                self.print_error(&format!("No hay pedido registrado con id {}", id_pedido));
            }
        }
        if let Some(pedido) = self.pedidos.get(&msg.id_pedido()).cloned() {
            let fut = async move {
                if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
                    let msg_clone = ProgresoPedido::new(EstadoPedido::Listo, msg.id_pedido());
                    if let Ok(serialized_msg) = serde_json::to_vec(&msg_clone) {
                        let mut data: Vec<u8> = Vec::new();
                        data.push(PROGRESO_PEDIDO_CODE);
                        data.extend(serialized_msg.len().to_be_bytes());
                        data.extend(serialized_msg);
                        let _ = socket.send_to(&data, pedido.0).await;
                    }
                };
            }
            .into_actor(self);
            return Box::pin(fut);
        }
        Box::pin(async move {}.into_actor(self))
    }
}

impl Handler<InfoRestaurantesInterno> for NodoRestaurante {
    type Result = ResponseActFuture<Self, ()>;

    /// Si llego al lider, ya recorrió el anillo, y se le envia `InfoRestaurantes` al comensal original que pidió la información.
    /// El resto añade su restaurante si esta a una distancia menor a MAX_DISTANCIA_RESTAURANTES, y reenvia el mensaje a siguiente nodo.
    fn handle(&mut self, msg: InfoRestaurantesInterno, _ctx: &mut Self::Context) -> Self::Result {
        self.print_cyan(&format!(
            "Recibido InfoRestaurantesInterno de comensal {:?}",
            msg.comensal,
        ));
        if self.local == self.lider.unwrap() {
            let comensal = *msg.address_comensal();
            let info_msg = InfoRestaurantes::new(msg.info_restaurantes().clone());
            self.print_blue(&format!(
                "Enviando InfoRestaurantesInterno a comensal {}",
                msg.comensal,
            ));
            Box::pin(
                async move {
                    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
                        if let Ok(serialized_msg) = serde_json::to_vec(&info_msg) {
                            let mut data: Vec<u8> = Vec::new();
                            data.push(INFO_RESTAURANTES_CODE);
                            data.extend(serialized_msg.len().to_be_bytes());
                            data.extend(serialized_msg);
                            let _ = socket.send_to(&data, comensal).await;
                        }
                    }
                }
                .into_actor(self),
            )
        } else {
            let restaurante_addr = self.restaurante.0.clone();
            let restaurante_id = self.restaurante.1;
            let siguiente = self.siguiente;

            Box::pin(
                async move {
                    if let Ok(ubicacion) = restaurante_addr.send(ObtenerUbicacion).await {
                        let mut info_restaurantes = msg.info_restaurantes().clone();

                        if ubicacion.distancia(msg.ubicacion_comensal())
                            < MAX_DISTANCIA_RESTAURANTES
                        {
                            info_restaurantes.insert(restaurante_id, ubicacion);
                        }

                        let new_msg = InfoRestaurantesInterno::new(
                            *msg.address_comensal(),
                            info_restaurantes,
                            *msg.ubicacion_comensal(),
                        );

                        Self::enviar_mensaje(INFO_RESTAURANTES_INTERNO_CODE, &new_msg, siguiente)
                            .await;
                    }
                }
                .into_actor(self),
            )
        }
    }
}

impl Handler<VerRestaurantes> for NodoRestaurante {
    type Result = ResponseActFuture<Self, ()>;

    /// Mensaje enviado por el comensal para ver los restaurantes que tiene cerca.
    ///
    /// Si este nodo es el líder, empieza a recorrer el anillo con el mensaje de InfoRestaurantesInterno.
    /// Si no lo es, reenvía la solicitud al líder.
    fn handle(&mut self, msg: VerRestaurantes, _ctx: &mut Self::Context) -> Self::Result {
        self.print_cyan(&format!(
            "Recibido VerRestaurantes de {:?}",
            msg.address_comensal(),
        ));
        let logger = self.logger.clone();
        match self.lider {
            Some(lider) => {
                if lider == self.local {
                    let restaurante_addr = self.restaurante.0.clone();
                    let restaurante_id = self.restaurante.1;
                    let siguiente = self.siguiente;
                    let fut = async move {
                        if let Ok(ubicacion) = restaurante_addr.send(ObtenerUbicacion).await {
                            let mut info_restaurantes = HashMap::new();

                            if ubicacion.distancia(msg.ubicacion_comensal())
                                < MAX_DISTANCIA_RESTAURANTES
                            {
                                info_restaurantes.insert(restaurante_id, ubicacion);
                            }

                            let info_msg = InfoRestaurantesInterno::new(
                                *msg.address_comensal(),
                                info_restaurantes,
                                *msg.ubicacion_comensal(),
                            );

                            let _ = logger.info(
                                &format!(
                                    "Respondiendo InfoRestaurantesInterno a {} (comensal: {:?})",
                                    siguiente,
                                    msg.address_comensal()
                                ),
                                colored::Color::Blue,
                            );

                            Self::enviar_mensaje(
                                INFO_RESTAURANTES_INTERNO_CODE,
                                &info_msg,
                                siguiente,
                            )
                            .await;
                        }
                    }
                    .into_actor(self);

                    Box::pin(fut)
                } else {
                    let fut = async move {
                        let _ = logger.info(
                            &format!("Reenviando VerRestaurantes al lider: {}", lider),
                            colored::Color::White,
                        );
                        Self::enviar_mensaje(VER_RESTAURANTES_CODE, &msg, lider).await;
                    }
                    .into_actor(self);

                    Box::pin(fut)
                }
            }
            None => {
                self.print_error("No hay lider, iniciando elección");
                let siguiente = self.siguiente;
                let local = self.local;
                let fut = async move {
                    Self::enviar_mensaje(
                        RING_ELECTION_CODE,
                        &RingElection::new(local.port()),
                        siguiente,
                    )
                    .await;
                    actix_rt::time::sleep(std::time::Duration::from_secs(10)).await;
                    let _ = Self::enviar_mensaje(VER_RESTAURANTES_CODE, &msg, local).await;
                }
                .into_actor(self);

                Box::pin(fut)
            }
        }
    }
}

impl Handler<Pedir> for NodoRestaurante {
    type Result = ResponseActFuture<Self, ()>;

    /// Si el pedido es para este restaurante, lo procesa localmente.
    /// Si es para otro restaurante, lo reenvía al líder para su distribución al correcto.
    fn handle(&mut self, msg: Pedir, ctx: &mut Self::Context) -> Self::Result {
        if msg.id_restaurante() != self.restaurante.1 {
            match self.lider {
                Some(lider) => {
                    if lider == self.local {
                        self.print_blue(&format!(
                            "Soy el lider, intentando reenviar Pedir a restaurante {}",
                            msg.id_restaurante()
                        ));
                        if let Some(objetivo) =
                            self.restaurantes.get(&msg.id_restaurante()).cloned()
                        {
                            self.print_blue(&format!(
                                "Enviando Pedir a restaurante {} en {}",
                                msg.id_restaurante(),
                                objetivo
                            ));

                            let fut = async move {
                                Self::enviar_mensaje(PEDIR_CODE, &msg, objetivo).await;
                            }
                            .into_actor(self);
                            return Box::pin(fut);
                        } else {
                            self.print_error(&format!(
                                "No tengo información del restaurante {}",
                                msg.id_restaurante()
                            ));
                        }
                    } else {
                        self.print_white(&format!("Reenviando Pedir al lider: {}", lider));
                        let fut = async move {
                            Self::enviar_mensaje(PEDIR_CODE, &msg, lider).await;
                        }
                        .into_actor(self);
                        return Box::pin(fut);
                    }
                }
                None => self.iniciar_votacion_nuevo_lider(ctx),
            }
        } else {
            self.print_cyan("Pedido para mi propio restaurante, procesando...");
            self.pedidos.insert(
                msg.id_pedido(),
                (*msg.dir_comensal(), msg.ubicacion_comensal),
            );
            let pedido =
                InfoRestaurantePedido::new(msg.id_pedido, msg.dir_comensal, msg.id_restaurante);
            self.restaurante.0.do_send(msg);
            if let Some(lider) = self.lider {
                return Box::pin(
                    async move {
                        Self::enviar_mensaje(
                            AGREGAR_PEDIDO_EN_CURSO_CODE,
                            &AgregarPedidoEnCurso::new(pedido),
                            lider,
                        )
                        .await;
                    }
                    .into_actor(self),
                );
            }
        }
        Box::pin(async move {}.into_actor(self))
    }
}

impl Handler<EliminarPedidoEnCurso> for NodoRestaurante {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: EliminarPedidoEnCurso, ctx: &mut Self::Context) -> Self::Result {
        let _ = self.logger.info(
            &format!(
                "Recibido EliminarPedidoEnCurso para restaurante {} y pedido {}",
                msg.id_restaurante(),
                msg.id_pedido()
            ),
            colored::Color::White,
        );
        let pedidos_clone = self.pedidos_en_curso.clone();
        let pedidos_a_eliminar: Vec<_> = pedidos_clone
            .iter()
            .filter(|info_restaurante_pedido| {
                if msg.id_pedido() == 0 {
                    return info_restaurante_pedido.id_restaurante() == msg.id_restaurante();
                }
                info_restaurante_pedido.id_restaurante() == msg.id_restaurante()
                    && info_restaurante_pedido.id_pedido() == msg.id_pedido()
            })
            .collect();

        self.pedidos_en_curso.retain(|info_restaurante_pedido| {
            if info_restaurante_pedido.id_pedido() == 0 {
                return info_restaurante_pedido.id_restaurante() != msg.id_restaurante();
            }
            info_restaurante_pedido.id_restaurante() != msg.id_restaurante()
                || info_restaurante_pedido.id_pedido() != msg.id_pedido()
        });

        for pedido in pedidos_a_eliminar {
            self.print_white(&format!(
                "Pedido {} eliminado, restaurante {}",
                pedido.id_pedido(),
                pedido.id_restaurante()
            ));

            let pedido_clone = pedido.clone();
            let fut = async move {
                if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
                    if let Ok(serialized_msg) = serde_json::to_vec(&ProgresoPedido::new(
                        EstadoPedido::Cancelado,
                        pedido_clone.id_pedido(),
                    )) {
                        let mut data: Vec<u8> = Vec::new();
                        data.push(PROGRESO_PEDIDO_CODE);
                        data.extend(serialized_msg.len().to_be_bytes());
                        data.extend(serialized_msg);
                        let _ = socket.send_to(&data, pedido_clone.dir_comensal()).await;
                    }
                }
            };

            ctx.spawn(fut.into_actor(self).map(|_result, _actor, _ctx| ()));
        }

        Box::pin(actix::fut::ready(()))
    }
}
impl Handler<PedidoTomado> for NodoRestaurante {
    type Result = ();

    /// Notifica al comensal cuando un repartidor ha tomado su pedido y se encuentra en camino.
    fn handle(&mut self, msg: PedidoTomado, ctx: &mut Self::Context) -> Self::Result {
        self.print_cyan(&format!(
            "Recibido PedidoTomado: {} para restaurante {}",
            msg.id_pedido(),
            msg.id_restaurante()
        ));

        if self.lider.is_none() {
            self.iniciar_votacion_nuevo_lider(ctx);
        }

        if msg.id_restaurante() != self.restaurante.1 {
            if let Some(lider) = self.lider {
                if self.local == lider {
                    if let Some(objetivo) = self.restaurantes.get(&msg.id_restaurante()).cloned() {
                        self.print_blue(&format!("Reenvia PedidoTomado a: {}", objetivo.port()));
                        let fut = async move {
                            Self::enviar_mensaje(PEDIDO_TOMADO_CODE, &msg, objetivo).await;
                        }
                        .into_actor(self);
                        ctx.spawn(fut);
                    }
                } else {
                    let fut = async move {
                        Self::enviar_mensaje(PEDIDO_TOMADO_CODE, &msg, lider).await;
                    }
                    .into_actor(self);
                    ctx.spawn(fut);
                }
            }
        } else if let Some(pedido) = self.pedidos.remove(&msg.id_restaurante()) {
            let id_pedido = msg.id_pedido();
            let id_restaurante = self.restaurante.1;
            let lider = self.lider.expect("imposible error");
            self.print_white("Avisando a comensal del progreso");
            let fut = async move {
                Self::enviar_mensaje(
                    ELIMINAR_PEDIDO_EN_CURSO_CODE,
                    &EliminarPedidoEnCurso::new(id_pedido, id_restaurante),
                    lider,
                )
                .await;
                if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
                    if let Ok(serialized_msg) =
                        serde_json::to_vec(&ProgresoPedido::new(EstadoPedido::EnCamino, id_pedido))
                    {
                        let mut data: Vec<u8> = Vec::new();
                        data.push(PROGRESO_PEDIDO_CODE);
                        data.extend(serialized_msg.len().to_be_bytes());
                        data.extend(serialized_msg);
                        let _ = socket.send_to(&data, pedido.0).await;
                    }
                }
            }
            .into_actor(self);
            ctx.spawn(fut);
        }
    }
}

/// Handler para mensajes de elección de líder (algoritmo de anillo).
impl Handler<RingElection> for NodoRestaurante {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: RingElection, _ctx: &mut Self::Context) -> Self::Result {
        self.print_cyan("Recibido RingElection");

        let mut msg = msg.clone();
        let mut seguir_enviando = true;
        let siguiente_clone = self.siguiente;
        if msg.tipo() == TIPO_ELECTION {
            if msg.puerto_original() == self.local.port() {
                msg.cambiar_a_coordinator();
                self.lider = Some(SocketAddrV4::new(
                    *self.local.ip(),
                    msg.puerto_maximo().unwrap(),
                ));
            } else {
                msg.agregar_puerto(self.local.port());
            }
        } else {
            self.lider = Some(SocketAddrV4::new(
                *self.local.ip(),
                msg.puerto_maximo().unwrap(),
            ));
            if msg.puerto_original() == self.local.port() {
                seguir_enviando = false;
            }
        }
        let fut = async move {
            if seguir_enviando {
                Self::enviar_mensaje(RING_ELECTION_CODE, &msg, siguiente_clone).await;
            }
        }
        .into_actor(self);
        Box::pin(fut)
    }
}

/// Handler para actualizar el nodo siguiente en el anillo.
impl Handler<ActualizarSiguiente> for NodoRestaurante {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: ActualizarSiguiente, _ctx: &mut Self::Context) -> Self::Result {
        self.print_cyan("Recibido ActualizarSiguiente");

        let ip = *self.local.ip();
        let rango = self.rango_nodos;
        let local_port = self.local.port();
        let actual_port = self.siguiente.port();

        let fut = async move {
            let range_size = (rango.1 - rango.0 + 1) as usize;
            let start_offset = (actual_port - rango.0) as usize;

            for i in 1..=range_size {
                let port_offset = (start_offset + i) % range_size;
                let siguiente_port = rango.0 + port_offset as u16;

                if siguiente_port == local_port {
                    continue;
                }

                let addr = SocketAddrV4::new(ip, siguiente_port);
                if TcpStream::connect(addr).await.is_ok() {
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
        .into_actor(self)
        .map(|nuevo_addr, actor, _ctx| {
            actor.siguiente = nuevo_addr;
        });

        Box::pin(fut)
    }
}

/// Handler para mensajes KeepAlive (latidos de corazón).
///
/// Se utilizan para detectar fallos en los nodos del anillo.
impl Handler<KeepAlive> for NodoRestaurante {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: KeepAlive, ctx: &mut Self::Context) -> Self::Result {
        let anterior = self.anterior;
        let lider = self.lider;
        let dir_emisor = msg.dir_emisor();
        if anterior.port() == msg.dir_emisor().port() {
            return Box::pin(async move {}.into_actor(self));
        }
        if self.lider.is_none() {
            self.iniciar_votacion_nuevo_lider(ctx)
        }
        let fut = async move {
            Self::enviar_mensaje(
                CAMBIAR_SIGUIENTE_CODE,
                &CambiarSiguiente::new(dir_emisor),
                anterior,
            )
            .await;
            if let Some(l) = lider {
                Self::enviar_mensaje(INFO_LIDER_CODE, &InfoLider::new(l), dir_emisor).await;
            }
        }
        .into_actor(self)
        .map(move |_, actor, _| {
            actor.anterior = msg.dir_emisor();
        });
        Box::pin(fut)
    }
}

/// Handler para mensajes de información del líder.
///
/// Actualiza la información local sobre quién es el líder actual del anillo.
impl Handler<InfoLider> for NodoRestaurante {
    type Result = ();

    fn handle(&mut self, msg: InfoLider, _ctx: &mut Self::Context) -> Self::Result {
        self.print_cyan(&format!("Recibido InfoLider: {}", msg.lider()));
        self.lider = Some(*msg.lider());
    }
}

/// Handler para mensajes informar cambios al siguiente nodo.
///
/// Se utiliza para actualizar el nodo siguiente en el anillo de comunicación, por ejemplo en caso de fallos.
impl Handler<CambiarSiguiente> for NodoRestaurante {
    type Result = ();

    fn handle(&mut self, msg: CambiarSiguiente, _ctx: &mut Self::Context) -> Self::Result {
        self.print_cyan(&format!("Recibido CambiarSiguiente: {}", msg.siguiente()));
        self.siguiente = *msg.siguiente();
    }
}

impl Handler<AgregarPedidoEnCurso> for NodoRestaurante {
    type Result = ();

    /// Agrega un pedido en curso a la lista de pedidos en curso del restaurante.
    fn handle(&mut self, msg: AgregarPedidoEnCurso, _ctx: &mut Self::Context) -> Self::Result {
        self.pedidos_en_curso.push(msg.pedido());
        self.print_white(&format!(
            "Pedido {} con id_restaurante {} agregado a pedidos en curso",
            msg.pedido().id_pedido(),
            msg.pedido().id_restaurante()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::estructuras_aux::ubicacion::Ubicacion;
    use crate::logger::log::AppType;
    use std::net::Ipv4Addr;
    use std::path::Path;

    #[actix_rt::test]
    async fn crear_nodo_restaurante() {
        let id = 11001;
        let addr_rest = Restaurante::new(
            id,
            Ubicacion::new(0, 0),
            None,
            1,
            Logger::new(Path::new("./"), AppType::Restaurante, id as u16, false).expect("REASON"),
        )
        .start();

        let _nodo = NodoRestaurante::new(
            (addr_rest, id),
            SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 0), id as u16),
            (11001, 11001),
            Ipv4Addr::new(127, 0, 0, 1),
            (11002, 11010),
            Logger::new(Path::new("./"), AppType::NodoRestaurante, id as u16, false)
                .expect("REASON"),
        );
    }
}
