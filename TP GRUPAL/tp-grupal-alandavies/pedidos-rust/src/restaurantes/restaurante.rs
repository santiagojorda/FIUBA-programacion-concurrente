use crate::estructuras_aux::estado_pedido::EstadoPedido;
use crate::logger::log::Logger;
use crate::mensajes::addr_nodo::AddrNodo;
use crate::mensajes::obtener_ubicacion::ObtenerUbicacion;
use crate::{
    estructuras_aux::{pedido::Pedido, ubicacion::Ubicacion},
    mensajes::pedido_a_cocinar::PedidoACocinar,
    restaurantes::cocina::{Cocina, PedidoCocido},
};
use std::collections::HashMap;

use crate::mensajes::{pedir::Pedir, progreso_pedido::ProgresoPedido};

use actix::dev::ToEnvelope;
use actix::{Actor, Addr, AsyncContext, Context, Handler, MessageResult};

pub struct Restaurante<A>
where
    A: Actor,
    A: Handler<ProgresoPedido>,
    <A as Actor>::Context: ToEnvelope<A, ProgresoPedido>,
{
    id: u32,
    ubicacion: Ubicacion,
    receptor_progreso: Option<Addr<A>>,
    pedidos_recibidos: HashMap<u32, Pedido>,
    cocinas: Vec<Addr<Cocina<Restaurante<A>>>>,
    cantidad_cocinas: usize,
    cocina_proxima: usize,
    logger: Logger,
}

impl<A> Actor for Restaurante<A>
where
    A: Actor,
    A: Handler<ProgresoPedido>,
    <A as Actor>::Context: ToEnvelope<A, ProgresoPedido>,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let restaurante_addr = ctx.address().clone();
        for _ in 0..self.cantidad_cocinas {
            let cocina_addr = Cocina::new(restaurante_addr.clone()).start();
            println!(
                "[Restaurante {}] Cocina iniciada addr: {:?}",
                self.id, cocina_addr
            );
            self.cocinas.push(cocina_addr);
        }
    }
}

impl<A> Restaurante<A>
where
    A: Actor,
    A: Handler<ProgresoPedido>,
    <A as Actor>::Context: ToEnvelope<A, ProgresoPedido>,
{
    pub fn new(
        id: u32,
        ubicacion: Ubicacion,
        receptor_progreso: Option<Addr<A>>,
        cantidad_cocinas: usize,
        logger: Logger,
    ) -> Self {
        Restaurante {
            id,
            ubicacion,
            receptor_progreso,
            pedidos_recibidos: HashMap::new(),
            cocinas: Vec::new(),
            cocina_proxima: 0,
            cantidad_cocinas,
            logger,
        }
    }

    fn print_cyan(&self, msg: &str) {
        let _ = self.logger.info(msg, colored::Color::Cyan);
    }

    fn print_error(&self, msg: &str) {
        let _ = self.logger.error(msg);
    }

    fn enviar_a_cocina(&mut self, pedido: Pedido) {
        if self.cocinas.is_empty() {
            self.print_error("No hay cocinas disponibles en el restaurante.");
            return;
        }

        let cocina = &self.cocinas[self.cocina_proxima];
        self.cocina_proxima = (self.cocina_proxima + 1) % self.cocinas.len();

        cocina.do_send(PedidoACocinar::new(pedido.id()));
    }
}

impl<A> Handler<PedidoCocido> for Restaurante<A>
where
    A: Actor,
    A: Handler<ProgresoPedido>,
    <A as Actor>::Context: ToEnvelope<A, ProgresoPedido>,
{
    type Result = ();

    /// Actualiza el estado del pedido y notifica al receptor de progreso.
    fn handle(&mut self, msg: PedidoCocido, _ctx: &mut Self::Context) -> Self::Result {
        match self.pedidos_recibidos.get_mut(&msg.id_pedido()) {
            Some(_pedido) => {
                if let Some(app_restaurante) = &self.receptor_progreso {
                    let progreso = ProgresoPedido::new(EstadoPedido::Listo, msg.id_pedido());
                    app_restaurante.do_send(progreso);
                    self.print_cyan(&format!(
                        "Receptor actualizado ID: {:?} , pedido listo",
                        msg.id_pedido()
                    ));
                } else {
                    self.print_error(
                        "App Restaurante no disponible para enviar progreso del pedido",
                    );
                }
            }
            _ => {
                self.print_error(&format!(
                    "No hay nodo asignado a restaurante {id}",
                    id = self.id
                ));
            }
        }
    }
}

impl<A> Handler<Pedir> for Restaurante<A>
where
    A: Actor,
    A: Handler<ProgresoPedido>,
    <A as Actor>::Context: ToEnvelope<A, ProgresoPedido>,
{
    type Result = ();

    /// Maneja una solicitud de nuevo pedido.
    ///
    /// Crea el pedido, lo almacena y lo envía a cocinar.
    /// Notifica al receptor de progreso sobre el nuevo estado.
    fn handle(&mut self, msg: Pedir, _ctx: &mut Self::Context) -> Self::Result {
        let nuevo_pedido = Pedido::new(
            msg.id_pedido(),
            EstadoPedido::Preparando,
            msg.monto(),
            msg.id_restaurante(),
            *msg.ubicacion_comensal(),
        );

        self.pedidos_recibidos
            .insert(msg.id_pedido(), nuevo_pedido.clone());

        self.enviar_a_cocina(nuevo_pedido);
        let progreso = ProgresoPedido::new(EstadoPedido::Preparando, msg.id_pedido());

        match &self.receptor_progreso {
            Some(addr) => {
                addr.do_send(progreso);
                self.print_cyan(&format!(
                    "Receptor actualizado con progreso ID: {:?}",
                    msg.id_pedido()
                ));
            }
            None => {
                self.print_error("App Restaurante no disponible para enviar progreso del pedido");
            }
        }
    }
}

impl<A> Handler<ObtenerUbicacion> for Restaurante<A>
where
    A: Actor,
    A: Handler<ProgresoPedido>,
    <A as Actor>::Context: ToEnvelope<A, ProgresoPedido>,
{
    type Result = MessageResult<ObtenerUbicacion>;

    /// Devuelve la ubicación del restaurante al receptor
    fn handle(&mut self, _msg: ObtenerUbicacion, _ctx: &mut Self::Context) -> Self::Result {
        self.print_cyan("Enviado Ubicacion");
        MessageResult(self.ubicacion)
    }
}

impl<A> Handler<AddrNodo<A>> for Restaurante<A>
where
    A: Actor + Send + 'static,
    A: Handler<ProgresoPedido>,
    <A as Actor>::Context: ToEnvelope<A, ProgresoPedido>,
{
    type Result = ();

    /// Establece el tipo de Actor que nos va a enviar mensajes y recibir las actualizaciones de progreso.
    fn handle(&mut self, msg: AddrNodo<A>, _ctx: &mut Self::Context) -> Self::Result {
        self.receptor_progreso = Some(msg.get_addr().clone());
        self.print_cyan("Receptor de progreso actualizado");
    }
}

#[cfg(test)]
mod test {
    use crate::logger::log::AppType;

    use super::*;
    use std::net::SocketAddrV4;
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use tokio::time::sleep;
    use tokio::time::Duration;

    /// TESTS
    /// Mock para logear el progreso de los pedidos
    pub struct MockReceptorProgreso {
        pub pedidos_preparando: Arc<Mutex<usize>>,
        pub pedidos_listos: Arc<Mutex<usize>>,
    }
    impl Actor for MockReceptorProgreso {
        type Context = Context<Self>;
    }

    impl Handler<ProgresoPedido> for MockReceptorProgreso {
        type Result = ();

        fn handle(&mut self, msg: ProgresoPedido, _ctx: &mut Self::Context) -> Self::Result {
            match msg.estado_pedido() {
                EstadoPedido::Preparando => {
                    println!("pedido nuevo {:?} preparando", msg.id_pedido());
                    *self.pedidos_preparando.lock().unwrap() += 1;
                }
                EstadoPedido::Listo => {
                    println!("pedido {:?} Listo", msg.id_pedido());
                    *self.pedidos_listos.lock().unwrap() += 1;
                    *self.pedidos_preparando.lock().unwrap() -= 1;
                }
                _ => {
                    println!(
                        "Estado de pedido no manejado en mock: {:?}",
                        msg.estado_pedido()
                    );
                }
            }
        }
    }

    /// Crea un mock y nos devuelve su addr para testear un mutex a pedidos listos y otro a preparando
    fn setup_mock_receptor_progreso() -> (
        Addr<MockReceptorProgreso>,
        Arc<Mutex<usize>>,
        Arc<Mutex<usize>>,
    ) {
        let pedidos_preparando_interno = Arc::new(Mutex::new(0));
        let pedidos_listos_interno = Arc::new(Mutex::new(0));
        let mock_pedidos_preparando = pedidos_preparando_interno.clone();
        let mock_pedidos_listos = pedidos_listos_interno.clone();
        let mock_receptor_addr = MockReceptorProgreso {
            pedidos_preparando: pedidos_preparando_interno,
            pedidos_listos: pedidos_listos_interno,
        }
        .start();
        (
            mock_receptor_addr,
            mock_pedidos_listos,
            mock_pedidos_preparando,
        )
    }

    fn setup_logger(id: u16) -> Logger {
        return Logger::new(Path::new("./"), AppType::NodoRestaurante, id, false).expect("REASON");
    }

    ///Inicia un restaurante normalmente
    #[actix_rt::test]
    async fn inicar_restautante() {
        println!(" ");
        println!("=================TESTS 1=================");
        let restaurante = Restaurante::<MockReceptorProgreso>::new(
            1,
            Ubicacion::new(1, 1),
            None,
            0,
            setup_logger(1),
        );
        assert_eq!(restaurante.cantidad_cocinas, 0);
        assert_eq!(restaurante.cocinas.is_empty(), true);
        assert_eq!(restaurante.receptor_progreso.is_none(), true);
        assert_eq!(restaurante.id, 1);
        assert_eq!(restaurante.ubicacion.latitud(), 1);
        assert_eq!(restaurante.ubicacion.longitud(), 1);

        let _ = restaurante.start();
    }

    ///Inicia un restaurante normalmente, pero agrega cocinas
    #[actix_rt::test]
    async fn inicar_restautante_con_concinas() {
        println!(" ");
        println!("=================TESTS 2=================");
        let restaurante = Restaurante::<MockReceptorProgreso>::new(
            1,
            Ubicacion::new(1, 1),
            None,
            5,
            setup_logger(1),
        );
        assert_eq!(restaurante.cantidad_cocinas, 5);
        assert_eq!(restaurante.cocinas.is_empty(), true); // las cocinas no se inician ahora
        assert_eq!(restaurante.receptor_progreso.is_none(), true);
        assert_eq!(restaurante.id, 1);
        assert_eq!(restaurante.ubicacion.latitud(), 1);
        assert_eq!(restaurante.ubicacion.longitud(), 1);

        let _ = restaurante.start();
    }

    ///Inicia un restaurante normalmente, le agrega un receptor y comprueba la creaccion
    #[actix_rt::test]
    async fn inicar_restautante_con_concinas_y_receptor() {
        println!(" ");
        println!("=================TESTS 3=================");
        let (mock_receptor_addr, _mock_pedidos_listos, _mock_pedidos_preparando) =
            setup_mock_receptor_progreso();
        let restaurante = Restaurante::<MockReceptorProgreso>::new(
            1,
            Ubicacion::new(1, 1),
            Some(mock_receptor_addr),
            5,
            setup_logger(1),
        );
        assert_eq!(restaurante.cantidad_cocinas, 5);
        assert_eq!(restaurante.cocinas.is_empty(), true); // las cocinas no se inician ahora
        assert_eq!(restaurante.receptor_progreso.is_some(), true);
        assert_eq!(restaurante.id, 1);
        assert_eq!(restaurante.ubicacion.latitud(), 1);
        assert_eq!(restaurante.ubicacion.longitud(), 1);
        let _ = restaurante.start();
    }

    ///Inicia un restaurante normalmente, comprueba que el receptor este funcinado una vez creado el restaurante
    #[actix_rt::test]
    async fn inicar_restautante_receptor_esta_disponible() {
        println!(" ");
        println!("=================TESTS 4=================");
        let (mock_receptor_addr, mock_pedidos_listos, mock_pedidos_preparando) =
            setup_mock_receptor_progreso();
        let restaurante = Restaurante::<MockReceptorProgreso>::new(
            1,
            Ubicacion::new(1, 1),
            Some(mock_receptor_addr),
            5,
            setup_logger(1),
        );
        assert_eq!(restaurante.receptor_progreso.is_some(), true);
        let _ = restaurante
            .receptor_progreso
            .clone()
            .unwrap()
            .send(ProgresoPedido::new(EstadoPedido::Preparando, 4))
            .await;
        let _ = restaurante
            .receptor_progreso
            .clone()
            .unwrap()
            .send(ProgresoPedido::new(EstadoPedido::Listo, 4))
            .await;
        let _ = restaurante
            .receptor_progreso
            .clone()
            .unwrap()
            .send(ProgresoPedido::new(EstadoPedido::Preparando, 2))
            .await;
        let _ = restaurante
            .receptor_progreso
            .clone()
            .unwrap()
            .send(ProgresoPedido::new(EstadoPedido::Preparando, 7))
            .await;

        sleep(Duration::from_millis(100)).await;
        assert_eq!(*mock_pedidos_listos.lock().unwrap(), 1);
        assert_eq!(*mock_pedidos_preparando.lock().unwrap(), 2);
        restaurante.start();
        assert_eq!(*mock_pedidos_listos.lock().unwrap(), 1);
        assert_eq!(*mock_pedidos_preparando.lock().unwrap(), 2);
    }

    ///Inicia un restaurante normalmente, comprueba que el receptor este funcinado una vez creado el restaurante
    #[actix_rt::test]
    async fn inicar_restautante_receptor_seteado_con_mensaje() {
        println!(" ");
        println!("=================TESTS 5=================");
        let (mock_receptor_addr, mock_pedidos_listos, mock_pedidos_preparando) =
            setup_mock_receptor_progreso();
        let restaurante = Restaurante::<MockReceptorProgreso>::new(
            1,
            Ubicacion::new(1, 1),
            None,
            5,
            setup_logger(1),
        );
        assert_eq!(restaurante.receptor_progreso.is_none(), true);
        let addr_rest = restaurante.start();
        let _ = addr_rest.send(AddrNodo::new(mock_receptor_addr.clone()));

        let _ = mock_receptor_addr
            .send(ProgresoPedido::new(EstadoPedido::Preparando, 4))
            .await;
        let _ = mock_receptor_addr
            .send(ProgresoPedido::new(EstadoPedido::Listo, 4))
            .await;
        let _ = mock_receptor_addr
            .send(ProgresoPedido::new(EstadoPedido::Preparando, 2))
            .await;
        let _ = mock_receptor_addr
            .send(ProgresoPedido::new(EstadoPedido::Preparando, 7))
            .await;

        sleep(Duration::from_millis(100)).await;
        assert_eq!(*mock_pedidos_listos.lock().unwrap(), 1);
        assert_eq!(*mock_pedidos_preparando.lock().unwrap(), 2);
    }

    ///
    #[actix_rt::test]
    async fn restaurante_cocina_pedidos() {
        println!(" ");
        println!("=================TESTS 6=================");

        let (mock_receptor_addr, mock_pedidos_listos, mock_pedidos_preparando) =
            setup_mock_receptor_progreso();
        let restaurante = Restaurante::new(1, Ubicacion::new(1, 1), None, 5, setup_logger(1));

        let addr_restaurante = restaurante.start();

        let _ = addr_restaurante
            .send(AddrNodo::new(mock_receptor_addr))
            .await;

        let _ = addr_restaurante
            .send(Pedir::new(
                3,
                400.0,
                1,
                Ubicacion::new(0, 0),
                SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 8080),
            ))
            .await;
        let _ = addr_restaurante
            .send(Pedir::new(
                4,
                400.0,
                1,
                Ubicacion::new(0, 0),
                SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 8080),
            ))
            .await;
        let _ = addr_restaurante
            .send(Pedir::new(
                5,
                400.0,
                1,
                Ubicacion::new(0, 0),
                SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 8080),
            ))
            .await;

        println!("[Restaurnate] con pedidos enviados");
        sleep(Duration::from_millis(200)).await;
        println!("Assert sobre pedidos");

        let mut pedidos_preparando = *mock_pedidos_preparando.lock().unwrap();
        let mut pedidos_listos = *mock_pedidos_listos.lock().unwrap();

        assert_eq!(pedidos_preparando, 3, "Pedidos preparando luego de enviar");
        assert_eq!(pedidos_listos, 0, "Pedidos listos luego de enviar");
        sleep(Duration::from_millis(2000)).await;
        println!("Assert sobre pedidos luego de esperar");

        pedidos_preparando = *mock_pedidos_preparando.lock().unwrap();
        pedidos_listos = *mock_pedidos_listos.lock().unwrap();

        assert_eq!(pedidos_preparando, 0, "Pedidos preparando luego esperar");
        assert_eq!(pedidos_listos, 3, "Pedidos listos luego esperar");
    }
}
