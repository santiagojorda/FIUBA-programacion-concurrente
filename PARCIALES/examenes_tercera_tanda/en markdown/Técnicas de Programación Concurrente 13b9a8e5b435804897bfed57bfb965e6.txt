# Técnicas de Programación Concurrente

# **Ejercicios de Parcial**

Modelos de Concurrencia

- Estado Mutable Compartido
- Paralelismo con Fork Join (caso especial Vectorización)
- Programación asincrónica
- Pasaje de mensajes (Canales)
- Actores

### 1° Cuatrimestre 2024

1. Revisando el diseño aplicado en algunos proyectos, se encontró el uso de las siguientes herramientas para resolver problemas de concurrencia. Para cada uno de los problemas enuncie ventajas o desventajas de utilizar la solución propuesta y menciona cual utilizaría usted.
- Renderizado de videos 3D en alta resolución, utilizando programación asincrónica.

La programación asincrónica nos es útil cuando tenemos que procesar muchas tareas de cómputo liviano (como las operaciones de I/O).

En este caso, tenemos una tarea de cómputo pesado, por lo que la programación asincrónica no es la mejor opción.

Para esta tarea convendría usar **vectorización, que es un caso particular del Fork Join**. Utilizando esta técnica se puede paralelizar lo más posible al cómputo y aprovechar el hardware al máximo.

- Aplicación que arma una nube de palabras a partir de la API de Twitter, utilizando barriers y mutex.

¿Tiene sentido utilizar el modelo de concurrencia de estado mutable compartido para este caso? No.

¿Qué hacen los barriers?

Permiten sincronizar varios threads para que se esperen entre sí hasta que todos lleguen a cierto punto. Esto es ineficiente para APIs ya que cada request puede demorarse tiempos variables.

¿Qué hacen los mutex?

Evitan que múltiples threads accedan al mismo tiempo a la sección crítica (recurso compartido). 

Siguendo este modelo de programación estamos más expuestos a tener condiciones de carrera, deadlocks que interferirían en el correcto funcionamiento de nuestra aplicación. Con estas herramientas garantizo el acceso seguro a los recursos a costa del rendimiento.

Lo correcto sería usar **programación asincrónica**, utilizando tareas asíncronas para cada request, que son operaciones de I/O. Estas tareas son muy livianas y evitan bloqueos innecesarios.

- Una aplicación para realizar una votación en vivo para un concurso de televisión, optimizada con Vectorización.

La vectorización permite que el procesador haga la misma operación sobre muchos datos al mismo tiempo, acelerando las tareas computacionales intensivas. ¿Me sirve para este caso? No.

La vectorización no resuelve problemas de concurrencia, sino que acelera los cálculos numéricos. 

Como estamos trabajando con recursos compartidos que forman parte de la sección crítica del sistema de votación, debemos usar el modelo de **estado mutable compartido**, de esta forma podemos asegurar la sincronización del conteo de los votos en tiempo real.

1. Programación asincrónica. Elija verdadero o falso y explique brevemente por qué:
- El encargado de hacer poll es el thread principal del programa. ❌

Esto es importante: el que ejecuta Poll es el Executor del runtime asincrónico y puede estar en cualquier hilo. El Executor es una abstracción que, por ejemplo, me ofrece Tokio en Rust para poder manejar las tareas asincrónicas.

```rust
async fn tarea() -> i64 {
	19
}

let future = tarea(); //No ejecuta, solo crea el Future
tokio::spawn(future); //Acá está el executor
```

- El método poll es llamado únicamente cuando la función puede progresar. ❌

El Poll se llama cada vez que el executor lo considera necesario, por ejemplo cuando hay una notificación de que un recurso podría estar disponible.

La llamada a Poll no garantiza que la función pueda progresar, ya que puede devolver Pending, indicando que aún no está listo.

- El modelo piñata es colaborativo. ✅

Modelo colaborativo 

Las tareas deciden cuándo ceder el control y no son interrumpidas de forma forzada.

El sistema cambia de tarea cuando una tarea dice: “Estoy esperando, podés pasar a otra”.

Modelo preventivo

El sistema puede interrumpir cualquier tarea en cualquier momento.

Es más robusto, pero requiere más sincronización.

El modelo piñata o Poll es colaborativo porque las tareas usan `await` para suspenderse voluntariamente, permitiendo que el Executor haga Poll a otras tareas listas para ejecutarse.

“Las tareas cooperan para que el sistema sea eficiente”

- La operación asincrónica inicia cuando se llama a un método declarado con async. ❌

Cuando se llama una función async no se ejecuta de forma inmediata. Retorna inmediatamente un Future que encapsula la operación pendiente. La ejecución comienza cuando el Executor hace Poll sobre el Future.

1. Para cada uno de los siguientes fragmentos de código indique si es o no es un busy wait. Justifique en cada caso (Nota: mineral y batteries_produced son locks).

Recordemos: ¿Qué es un busy wait? Es una espera activa, pregunto activamente si un recurso está disponible para poder continuar.

1° caso: ❌ No es busy wait.

```rust
for _ in 0..MINERS {
	let lithium = Arc::clone(&mineral);
	thread::spawn(move || loop {
		let mined = rand::thread_rng().gen();
		let random_result: f64 = rand::thread_rng().gen();
		*lithium.write().expect("failed to mine") += mined;
		thread::sleep(Duration::from_millis((5000 as f64 * random_result) as u64));
	})
}
```

¿Qué hace el código?

- Tenemos threads para los `MINERS` que simulan una operación sobre un recurso compartido `lithium` .
- Se hace un `write()` que espera a que el recurso esté libre para poder sumar el valor del resultado.
- Luego el thread hace un sleep, liberando el CPU. No ocupa el procesador mientras espera.

2° caso: ✅ Sí es busy wait

```rust
for _ in 0..MINERS {
	let lithium = Arc::clone(&mineral);
	let batteries_produced = Arc::clone(&resources);
	thread::spawn(move || loop {
		let mut lithium = lithium.write().expect("failed");
		if lithium >= 100 {
			lithium -= 100;
			batteries_produced.write().expect("failed to produce") += 1
		}
		thread::sleep(Duration::from_millis(500));
})}
```

¿Qué hace el código?

- Nuevamente thread para cada MINER.
- Dos recursos compartidos: lithium y batteries_produced.
- Se hace un write() esperando a que lithium esté libre para: preguntar por su valor y hacer una operación sobre el mismo. El thread queda bloqueado hasta que el recurso esté disponible y mientras tanto no hace nada. Eso es un busy wait.
- El sleep que se encuentra después es de 500 mS, tiempo bastante corto. Si lithium no cambia en mucho tiempo, estoy intentando al pepe cada 500 mS.
1. Dada la siguiente estructura, nombre si conoce una estructura de sincronización con el mismo comportamiento. Indique posibles errores en la implementación.

```rust
pub struct SynchronizationStruct {
	mutex: Mutex<i32>,
	cond_var: Condvar,
}

impl SynchronizationStruct {

	pub fn new(size: u16) -> SynchronizationStruct {
		SynchronizationStruct {
		mutex: Mutex::new(size),
		cond_var: Condvar::new(),
	}}
	
	pub fn function_1(&self) {
		let mut amount = self.mutex.lock().unwrap();
		if *amount <= 0 {
			amount = self.cond_var.wait(amount).unwrap();
		}
		*amount -= 1;
	}
	
	pub fn function_2(&self) {
		let mut amount = self.mutex.lock().unwrap();
		*amount += 1;
		self.cond_var.notify_all();
		}
}
```

La estructura implementada es un semáforo construido por monitores. El cual tiene 2 estados internos:

- V: entero no negativo: contador de recursos disponibles. Inicialmente comienza con un valor mayor a cero.
- L:  conjunto de procesos esperando. Inicia vació.

wait() “obtener acceso”

Le resta 1 al contador

Si V > 0 → V -= 1

Si V ≤ 0 → El proceso se bloquea y se agrega a L

signal() “liberar”

Si L está vacío → V += 1

Sino, se remueve un proceso de L y se lo pone en estado Ready (no se define cúal)

Entonces, ¿qué problema tiene el código? Veamos las funciones y las implementaciones de las mismas.

La `function_1` es el `wait()`

```rust
	pub fn function_1(&self) {
		let mut amount = self.mutex.lock().unwrap();
		if *amount <= 0 {
			amount = self.cond_var.wait(amount).unwrap();
		}
		*amount -= 1;
	}
```

Acá hay un gran problema, el `if *amount <= 0` . Usando esa lógica, nuestra función wait() funcionaría de la siguiente manera:

- Espera una vez si amount ≤ 0. Pero el método Condvar::wait() en Rust puede retornar incluso si nadie hizo notify() (spurious wakeup).
- Si se despierta cuando amount sigue siendo 0, el código sigue y hace *amount -= 1, lo cual rompe la invariante del semáforo (que debe mantenerse en amount >= 0).

¿Cómo se arregla? cambiando el if por un while.

```rust
pub fn function_1(&self) {
	let mut amount = self.mutex.lock().unwrap();
	while *amount <= 0 {
		amount = self.cond_var.wait(amount).unwrap();
	}
	*amount -= 1;
}

```

De esta forma, se pregunta cada vez que el thread despierta si el amount ≥ 0 o no.

La `function_2` es `signal()`

```rust
	pub fn function_2(&self) {
		let mut amount = self.mutex.lock().unwrap();
		*amount += 1;
		self.cond_var.notify_all();
		}
}
```

Acá solamente un pequeño detalle: el método `notify_all()`. No tiene sentido despertar a todos los threads, ya que uno solo será el que tome el control del recurso, puede usarse `notify_one()` .

1. Dados la siguiente red de Petri y fragmento de código, indique el nombre del problema que modelan. Indique si la implementación es correcta o describa cómo mejorarla.

![image.png](image.png)

Problema: Productor-Consumidor con buffer finito.

¿Cómo funciona la Red de Petri de este problema?

![image.png](image%201.png)

Veamos el código que nos proponen:

```rust
fn main() {
	let sem = Arc::new(Semaphore::new(0));
	let buffer = Arc::new(Mutex::new(Vec::with_capacity(N)));
	let sem_cloned = Arc::clone(&sem);
	let buf_cloned = Arc::clone(&buffer);
	let t1 = thread::spawn(move || {
	loop {
		// heavy computation
		let random_result: f64 = rand::thread_rng().gen();
		thread::sleep(Duration::from_millis((500 as f64 *
		random_result) as u64));
		buf_cloned.lock().expect("").push(random_result);
		sem_cloned.release()
	}
	});
	let sem_cloned = Arc::clone(&sem);
	let buf_cloned = Arc::clone(&buffer);
	let t2 = thread::spawn(move || {
		loop {
			sem_cloned.acquire();
			println!("{}", buf_cloned.lock().expect("").pop());
		}
	});
	t1.join().unwrap();
	t2.join().unwrap();
}
```

Este código serviría para el problema Productor-Consumidor con buffer infinito. Ya que nunca se pregunta por el espacio de tamaño N del buffer.  Para poder convertir el problema a buffer finito, debemos agregar otro semáforo más que indique si el buffer está lleno o no.

Productor

notFull.acquire()

producir()

lock buffer

push(producto)

notEmpty.release()

Consumidor

notEmpty.acquire()

lock buffer

consumir()

notFull.release()

## 2° Cuatrimestre 2024

1. Para cada uno de los siguientes fragmentos de código indique si es o no es un busy wait. Justifique en cada caso.

1° caso: ✅ Sí es busy wait

![image.png](image%202.png)

¿Qué hace el código? Es un loop.

- Intenta una conexión.
- Si puede, se conecta y escribe un mensaje.
- Si no puede, duerme durante un determinado tiempo y va a volver a intentar la conexión

Acá el sleep puede confundirse con liberar los recursos del CPU, sin embargo solo funciona para esperar un determinado tiempo para volver a intentar la conexión. Esto es un busy wait dado que mientras no pueda conectarse, está esperando activamente de forma indefinida hasta que logre la conexión. También podemos pensar que el connect gasta recursos del CPU (aunque no de forma intensiva.

Igual es dudoso, no es un busy wait puro, pero con una buena justificación va bien.

2° caso:  ✅ Sí es busy wait 

![image.png](image%203.png)

¿Qué hace el código? Es un loop.

- Tenemos un recurso compartido pending_acks que es items.
- Mientras i (que arranca en 0) es menor al largo de items: si cada item no está expirado, pregunta si es del tipo ACK, si es así, lo remueve, envía un resultado y lo corta. Si está expirado el item, suma uno a i y continúa.

Observemos que constantemente tenemos una verificación de una condición que está ocupando recursos de CPU, si esa condición no se cumple nunca, el thread nunca trabaja activamente. 

El thread está activamente dando vueltas esperando que llegue un ACK vencido.

Estos casos según leí se llaman “Polling activo con sleep”:

- Revisa el estado de algo cada vez que se despierta, buscando una condición.
- Esa condición podría tardar mucho o no cumplirse nunca.
- **No hay ningún mecanismo de notificación real** (como `Condvar`, canal, evento, etc.) que **despierte al hilo cuando hay cambios** en el recurso.

3° caso:  ❌ No es busy wait.

![image.png](image%204.png)

¿Qué hace el código?

- Un thread por cada MINER.
- En un loop, genera un random e intenta escribirlo con write() sobre el recurso compartido copper (&resource).
- Genera una demora que efectúa luego con el sleep.

¿Es un busy wait? No parece.  No tenemos ninguna condición dentro un bucle que defina el trabajo del thread.

Acá el sleep actúa liberando los recursos del CPU y no como un “espero un X tiempo y pruebo devuelta”

1. Modelar una Red de Petri para el problema del Lector-Escritorsin preferencia. Luego, modele una solución que contemple preferencia de escritura.

**Lector-Escritor sin preferencia**

![image.png](image%205.png)

Para que un escritor escriba:

- No debe haber otro ni leyendo ni escribiendo
Para que un lector lea:
- No debe haber otro escribiendo

**Lector-Escritor con preferencia en escritores**

![image.png](image%206.png)

Para que un lector lea:

- No tiene que estar nadie escribiendo
- No tiene que estar un escritor esperando
1. Se quiere abrir un restaurante en el barrio de San Telmo. Seespera que los clientes lleguen y sean atendidos por alguno de los mozos de turno, cada uno de los cuales tomará los pedidos de cada mesa, los notificará a la cocina y luego seguirá tomando otros pedidos. Como la cocina es chica los cocineros pueden entrar a buscar los ingredientes al depósito de a uno a la vez, y buscar entre los distintos alimentos les puede llevar un tiempo variable. Cuando los cocineros hayan terminado de preparar un pedido deben notificar a los mozos para que lo lleven a la mesa. Además, los mozos deben estar disponibles para cobrarle a los clientes. Diseñe el sistema utilizando el modelo de actores, y para cada entidad defina cuáles son los estados internos y los mensajes que intercambian.

Actores + estados internos:

Cliente:

- Comida: Pedido
- Mozo: <Addr<Mozo>> Mozo asignado al cliente
- Recepción: <Addr<Recepción>>

Recepción:

- Mozos: queue[<Addr<Mozo>>] Cola de mozos esperando atender

Cocina:

- Cocineros: queue[<Addr<Cocinero>>] Cola de cocineros esperando cocinar

Cocinero:

- Pedidos: Map<Pedido, <Addr<Mozo>>>

Deposito:

- Cocineros: queue[<Addr<Cocinero>>] Cola de cocineros esperando ingresar al depósito
- ocupado: bool

Mozo:

- Pedidos: Map<<Addr<Cliente>, Status> Por cada cliente, el status de su pedido
- Cocina: <<Addr<Cocina>

Estructura auxiliar:

Pedido:

- Ingredientes: vec[ingredientes]
- Status

Mensajes:

Cliente → Recepción

- Llegue

Recepción → Mozo

- Tomar_pedido(<Addr<Cliente>) → Pedido - Forwarding del mensaje al cliente para tomarle el pedido

Mozo → Cocina

- Notificar_pedido(<Addr<Mozo>, pedido) - La cocina toma el primer cocinero de la cola y le notifica el pedido a través del forwarding del mismo mensaje, ahí el cocinero recibe la característica del pedido.

Cocinero → Depósito

- Ingresar(<Addr<Cocinero>)

Este mensaje puede resolverse de forma recursiva de la siguiente manera (explicado en pseudocódigo):

```
ingresar(cocinero)
	ocupado?
		agregar cocinero a la cola de espera
	si no
		ocupado = true
		al cocinero -> envío msj de acceso concedido
		liberar()

liberar()
	si la cola de espera no está vacía:
		siguiente = sacar de la cola a un cocinero
		al siguiente -> envío msj de acceso concedido
		liberar()
	si no:
		ocupado = false	 		
```

De esta forma podemos manejar el acceso exclusivo del depósito.

Cocinero → Mozo

- Pedido_listo() → Comida (pedido ahora con status de terminado) Reenvía ese mensaje con el pedido listo al cliente para que lo consuma.

Cliente → Mozo

- Pedir_cuenta() → Cuenta

Cliente → Mozo

- Pagar_cuenta(dinero) → Saludo

Estos ejercicios son horribles.

1. Verdadero o Falso. Justifique
- Procesos, hilos y tareas asincrónicas poseen espacios de memoria independientes. ❌

Los threads comparten el mismo espacio de memoria que los procesos a los cuales pertenecen y las tareas asincrónicas comparten el stack del thread donde se ejecutan.

- El scheduler del sistema operativo puede detener una tarea asincrónica puntual y habilitar la ejecución de otra para el mismo proceso. ❌

Lo que maneja la programación asincrónica es el Executor, es una abstracción que por ejemplo nos da el runtime de Tokio en Rust para orquestar las tareas asincrónicas.

- Tanto los threads, como las tareas asincrónicas disponen de un stack propio. ❌

Como escribí anteoriormente, las tareas asincrónicas comparten el espacio de memoria del thread donde se ejecutan.

- En un ambiente de ejecución con una única CPU, un conjunto de hilos de procesamiento intensivo tomarán un tiempo de ejecución significativamente menor a un conjunto de tareas
asincrónicas que ejecuten el mismo procesamiento. ❌

Si hablamos de una única CPU, la diferencia de tiempo de ejecución no será tan evidente, ya que no hay paralelismo real. De todas formas, sabemos que las tareas asincrónicas son mucho más livianas que los hilos, así que podrían ser más eficientes porque producen menos overhead en el SO.

1. Describa y justifique con que modelo de concurrencia modelaría la implementación para cada uno de los siguientes casos de uso
- Convertir un conjunto extenso de archivos de .DOC a .PDF

Si suponemos que los archivos se encuentran en memoria, podemos utilizar el modelo de paralelismo Fork Join. Con este modelo logramos dividir las tareas en subtareas, resolverlas y luego combinar el resultado final. Es parecido al TP N°1.

- El backend para una aplicación de preguntas & respuestas competitiva al estilo Menti o Kahoot.

Acá tenemos múltiples clientes interactuando al mismo tiempo en la aplicación. Esas interacciones son llamadas a la API, es decir, operaciones de I/O. Para estos casos lo ideal es usar programación asincrónica, que son más livianas que los threads. Además algo que me puede definir entre usar un modelo de paralelismo con threads y con tareas asincrónicas es preguntarme ¿Necesito un thread vivo siempre? En este caso no, así que voy por el modelo asincrónico.

Hilos vivos = Fork-Join,
Tareas suspendibles sin hilos dedicados = Async/Await

- Una memoria caché utilizada para reducir la cantidad de requests en un servidor web a una base de datos.

En este caso, conviene usar el modelo de estado mutable compartido, dado que si suponemos que esta memoria caché es accedida por múltiples threads o tareas, debemos asegurarnos la sincronización y la integridad de los recursos compartidos en la zona crítica.

- Una API HTTP que ejecuta un modelo de procesamiento de lenguaje natural para clasificar el sentimiento de un mensaje.

Para un modelo de procesamiento de lenguaje natural que requiere ser entrenado utilizando muchos datos (muchas requests) la mejor opción es usar programación asincrónica porque las tareas son más livianas y rápidas, permitiendo un mejor desempeño de la API para este caso.

## 2° Cuatrimestre 2023

1. Describa cuál modelo utilizar para las siguientes situaciones (visto en la **clase de repaso**):
- Cálculo de matrices para modelo de redes neuronales.

Fork Join? Sí, pero más específico vectorización. Como tenemos un problema (producto de matrices) que podemos dividirlo en subproblemas (cálculos de fila y columna) para luego juntar los resultados,  este modelo sería el correcto.

Estado mutable compartido? Muy forzado, ¿realmente tengo que compartir algo? Además podría provocarme condiciones de carrera, deadlocks, bugs al pepe.

- Solicitudes a distintas APIs y combinar el resultado.

Las llamadas a las diferentes APIs se realiza con tareas asincrónicas donde cada una esperará (con await) a que terminen todas para que luego puedan combinarse los resultados.

Imaginemos que levantamos N tareas asincrónicas (N llamadas a APIs - Requests - Operaciones I/O) en un único thread. Con el Executor (con el Runtime de Tokio), esas tareas multiplexan el uso de hilos de ejecución. Además, las tareas asincrónicas son mucho más ligeras.

¿Por qué no Fork Join? Hay tiempo muerto donde no hay threads trabajando mientras que esperan a que terminen otros requests.

- Leer Log de una pagina que es muy visitada / Procesar logs de un sitio web muy concurrido

Acá depende mucho de qué interpreta cada uno. 

Suponiendo que tengo todos los logs en un archivo en memoria, podría usar un modelo de Fork Join. Devuelta lo mismo: problema → subproblemas → junto.

Pasaje de mensajes? Puede ser también, si interpreto que lo que quiero hacer es ir escribiendo los logs.

Estado mutable compartido? ¿Tengo algo que compartir? Depende la interpretación.

- Acceder al BackEnd de un videojuego

Acá interpreto que quiero acceder como jugador al juego, ahí tenemos entonces 2 opciones que se me ocurren:

1. Actores: cada jugador es un actor y trabaja con mensajes.
2. Estado mutable compartido: podríamos tener recursos compartidos en la sección crítica que sea necesario asegurar la integridad de los mismos si tenemos un acceso concurrente de múltiples jugadores al mismo tiempo. Si tomamos la consideración que el juego es multijugador ahí tiene mucho más sentido esto.
- Backend de un editor colaborativo en línea (no estaba en el parcial, pero lo vimos en el repaso)

Acá de una estado mutable compartido. Necesitamos el uso de herramientas de sincronización para el acceso a la sección crítica por parte de múltiples entidades.

1. Armar un Semáforo usando Monitores.

Mutex

Protege un dato compartido, evitando accesos simultáneos.

Condvar

Permite a los threads esperar (con wait()) y ser notificados cuando la condición se cumple.

Si junto Mutex + Condvar obtengo un Monitor, que es una abstracción de alto nivel que provee:

- Exclusión mutua automática
- Condición para esperar o notificar
- Encapsulamiento del estado compartido

¿Qué hace un semáforo?

Tiene 2 estados internos:

- V: entero no negativo: contador de recursos disponibles. Inicialmente comienza con un valor mayor a cero.
- L:  conjunto de procesos esperando. Inicia vació.

wait() “obtener acceso”

Le resta 1 al contador

Si V > 0 → V -= 1

Si V ≤ 0 → El proceso se bloquea y se agrega a L

signal() “liberar”

Si L está vacío → V += 1

Sino, se remueve un proceso de L y se lo pone en estado Ready (no se define cúal)

```rust
pub struct semaforo {
	recursos: Mutex<i32>,
	condvar: Condvar,
}

pub fn new(K: i32) -> Self {
	Self {
		recursos: Mutex::new(K),
		condvar: Condvar::new(),
	}
}

pub fn wait (&Self) {
	let mut recursos_compartidos = self.recursos.lock().unwrap();
	while recursos_compartidos == 0 {
		recursos_compartidos = self.condvar.wait(recursos_compartidos).unwrap();
	}
	recursos_compartidos -= 1;
}	

pub fn signal (&Self) {
	let mut recursos_compartidos = self.recursos.lock().unwrap();
	*recursos_compartidos += 1;
	self.condvar.notify_one();
}
```

1. Dibujar la red de Petri para el problema de productor-consumidor con buffer acotado.

![image.png](image%201.png)