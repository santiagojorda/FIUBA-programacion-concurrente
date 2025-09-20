use crate::mensajes::pedido_a_cocinar::PedidoACocinar;
use actix::dev::ToEnvelope;
use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, WrapFuture};
use core::time;
use std::collections::VecDeque;
use tokio::time::sleep;

#[derive(Message)]
#[rtype(result = "()")]
pub struct PedidoCocido {
    id_pedido: u32, // Renombre a singular para claridad
}

impl PedidoCocido {
    pub fn id_pedido(&self) -> u32 {
        self.id_pedido
    }
}

/// Mensaje para Cocina: Iniciar/Continuar Cocción
/// Este mensaje es para uso interno para manejar el ciclo de cocción
#[derive(Message)]
#[rtype(result = "()")]
struct IniciarCoccion;

pub struct Cocina<A>
where
    A: Actor,
    A: Handler<PedidoCocido>,
{
    pedidos: VecDeque<u32>, // Usamos VecDeque para eficiencia al quitar elementos del frente
    cocina_en_uso: bool,
    recepcion: Addr<A>, // Asumo que Restaurante es otro Actor
}

impl<A> Cocina<A>
where
    A: Actor,
    A: Handler<PedidoCocido>,
    <A as Actor>::Context: ToEnvelope<A, PedidoCocido>,
{
    pub fn new(recepcion: Addr<A>) -> Self {
        Cocina {
            pedidos: VecDeque::new(),
            cocina_en_uso: false,
            recepcion,
        }
    }
}
impl<A> Actor for Cocina<A>
where
    A: Actor,
    A: Handler<PedidoCocido>,
{
    type Context = Context<Self>;
}

/// Implementación del Handler para `PedidoNuevo`
impl<A> Handler<PedidoACocinar> for Cocina<A>
where
    A: Actor,
    A: Handler<PedidoCocido>,
    <A as Actor>::Context: ToEnvelope<A, PedidoCocido>,
{
    type Result = ();

    fn handle(&mut self, msg: PedidoACocinar, ctx: &mut Self::Context) -> Self::Result {
        println!("Cocina: Recibido nuevo pedido #{}", msg.id_pedido());

        self.pedidos.push_back(msg.id_pedido());

        if !self.cocina_en_uso {
            self.cocina_en_uso = true;
            let self_addr = ctx.address().clone();
            actix::spawn(async move {
                self_addr.send(IniciarCoccion {}).await.ok();
            });
        } else {
            println!("Cocina: Ya en uso, pedido #{} en cola.", msg.id_pedido());
        }
    }
}

/// Implementación del Handler para `IniciarCoccion`
impl<A> Handler<IniciarCoccion> for Cocina<A>
where
    A: Actor,
    A: Handler<PedidoCocido>,
    <A as Actor>::Context: ToEnvelope<A, PedidoCocido>,
{
    type Result = ();
    fn handle(&mut self, _msg: IniciarCoccion, ctx: &mut Self::Context) -> Self::Result {
        let recepcion_addr = self.recepcion.clone();
        let self_addr = ctx.address();

        let pedidos_a_cocinar: Vec<u32> = self.pedidos.drain(..).collect();

        if pedidos_a_cocinar.is_empty() {
            println!("Cocina: No hay pedidos para cocinar. Marcando cocina como libre.");
            self.cocina_en_uso = false; // Liberar la cocina
            return;
        }

        println!(
            "Cocina: Iniciando cocción para {} pedidos.",
            pedidos_a_cocinar.len()
        );

        // Lanzar una tarea asíncrona para procesar los pedidos
        ctx.spawn(
            async move {
                for id_pedido in pedidos_a_cocinar {
                    println!("Cocina: Cocinando pedido #{}...", id_pedido);

                    sleep(time::Duration::from_millis(750)).await;
                    recepcion_addr.do_send(PedidoCocido { id_pedido });
                    println!(
                        "Cocina: Pedido #{} listo y enviado al Restaurante.",
                        id_pedido
                    );
                }
                self_addr.send(IniciarCoccion {}).await.ok(); // ok() para ignorar errores si el actor ya paró
            }
            .into_actor(self),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_rt;
    use ntest::assert_true;
    use std::sync::{Arc, Mutex};
    use tokio::time::Instant;
    /// TESTS
    ///
    /// Mock para testear restautantes
    pub struct MockRestaurante {
        pub cocidos_recibidos: Arc<Mutex<VecDeque<u32>>>,
    }

    impl Actor for MockRestaurante {
        type Context = Context<Self>;
    }

    impl Handler<PedidoCocido> for MockRestaurante {
        type Result = ();

        fn handle(&mut self, msg: PedidoCocido, _ctx: &mut Self::Context) -> Self::Result {
            println!(
                "[MockRestaurante]: Recibido PedidoCocido #{}",
                msg.id_pedido()
            );
            let mut pedidos = self.cocidos_recibidos.lock().unwrap();
            pedidos.push_back(msg.id_pedido());
        }
    }

    /// Crea un mock y nos devuelve su addr y un lock para testear los pedidso
    fn setup_mock_restaurante() -> (Addr<MockRestaurante>, Arc<Mutex<VecDeque<u32>>>) {
        let cocidos_recibidos = Arc::new(Mutex::new(VecDeque::new()));
        let mock_restaurante_state = cocidos_recibidos.clone();
        let mock_restaurante_addr = MockRestaurante { cocidos_recibidos }.start();
        (mock_restaurante_addr, mock_restaurante_state)
    }

    /// Iniciamos la cocina con valores un retaurante mock
    #[actix_rt::test]
    async fn test_inicar_cocina() {
        println!(" ");
        println!("=================TESTS 1=================");
        let (mock_restaurante_addr, _cocidos_recibidos) = setup_mock_restaurante();
        let _cocina = Cocina::new(mock_restaurante_addr);
    }

    /// Iniciamos la cocina con valores un retaurante mock y eviamos pedidos
    #[actix_rt::test]
    async fn test_enviar_peidiodos_cocina() {
        println!(" ");
        println!("=================TESTS 2=================");

        let (mock_restaurante_addr, cocidos_recibidos) = setup_mock_restaurante();
        let addr_cocica = Cocina::new(mock_restaurante_addr).start();

        let _ = addr_cocica.send(PedidoACocinar::new(45)).await;
        let _ = addr_cocica.send(PedidoACocinar::new(432)).await;
        let _ = addr_cocica.send(PedidoACocinar::new(232)).await;

        sleep(time::Duration::from_millis(2000)).await; // margen para procesado

        //Chequeamos que se procesaron todos los pedidos
        assert_eq!(cocidos_recibidos.lock().unwrap().len(), 3);
    }

    #[actix_rt::test]
    async fn test_pedidos_enviados_mantienen_orden() {
        println!(" ");
        println!("=================TESTS 3=================");

        let (mock_restaurante_addr, cocidos_recibidos) = setup_mock_restaurante();
        let addr_cocica = Cocina::new(mock_restaurante_addr).start();

        let _ = addr_cocica.send(PedidoACocinar::new(1)).await;
        let _ = addr_cocica.send(PedidoACocinar::new(2)).await;
        let _ = addr_cocica.send(PedidoACocinar::new(3)).await;
        let _ = addr_cocica.send(PedidoACocinar::new(4)).await;

        sleep(time::Duration::from_millis(2000)).await; // margen para procesado

        //Chequeamos el orden de los pedidos
        assert_true!(cocidos_recibidos.lock().unwrap().iter().is_sorted());
    }

    #[actix_rt::test]
    async fn test_pedidos_enviados_no_bloquean() {
        println!(" ");
        println!("=================TESTS 4=================");

        let (mock_restaurante_addr, _cocidos_recibidos) = setup_mock_restaurante();
        let addr_cocica = Cocina::new(mock_restaurante_addr).start();

        let tiempo = Instant::now();

        let _ = addr_cocica.send(PedidoACocinar::new(1)).await;
        let _ = addr_cocica.send(PedidoACocinar::new(2)).await;
        let _ = addr_cocica.send(PedidoACocinar::new(3)).await;
        let _ = addr_cocica.send(PedidoACocinar::new(4)).await;

        //chequeamos que el tiempo sea menor a un pedido
        assert_true!(tiempo.elapsed() <= time::Duration::from_millis(250));
        println!("Tiempo en ejecucion del tests: {:?} ", tiempo.elapsed());
    }
}
