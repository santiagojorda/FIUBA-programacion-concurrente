#  Enunciado evaluacion parcial 2C 2023 

## Pregunta 1

### Enunciado

Determine y justifique cuál modelo de concurrencia convendría más utilizar para solucionar cada uno de los siguientes problemas:

1. Cálculo de multiplicacion entre matrices para entrenar redes neuronales.
2. Programa que realiza multiples requests a APIs y al finalizar combina los resultados.
3. Procesamiento de logs de acceso de un sitio web muy concurrido.
4. Backend de un juego en linea.

### Respuesta

1. Fork-join, son un montón de tareas de cómputo muy pesado y fácilmente paralelizables.
2. Asíncrona, porque puede haber bastante tiempo de espera entre las requests. Depende de que se tenga que hacer para combinar las requests podría ser conveniente aplicar fork-join para la parte final en específico
3. Asíncrona para obtener los logs, y depende del tipo de procesamiento necesario podría convenir usar fork-join (p. ej. el tp1 que era tal cual procesar logs de sitios webs concurridos con fork-join).
4. Me suena que await o algo más estilo reactive, aunque el tp de taller era un juego en linea y el back estaba hecho con fork-join así que no es imposible.

## Pregunta 2

### Enunciado

Desarrolle pseudo codigo de Rust para solucionar el siguiente problema:

Se quiere medir una aproximacion del tiempo de latencia que experimenta un servicio de conexión a internet.
Para ello se requiere desarrollar un programa que realiza peticiones concurrentemente a distintos sitios web que se obtienen como lineas a partir de un archivo (El archivo tiene 100 lineas).
El programa deberá realizar una peticion por sitio, de manera concurrente, y calcular el tiempo que
tarda la peticion en ser respondida. Al finalizar se debe promediar el tiempo de demora de todos los sitios.
Para modelar las request utilice `thread::sleep` con una duración random en segundos.
Tener en cuenta que si bien se quiere optimizar las peticiones de forma concurrente, se debe tener un
limite de N peticiones para no saturar la red y para limitar el numero de threads.

Realizar el DAG del problema

### Respuesta

```Rust
use rayon::prelude::*
use rayon::ThreadPoolBuilder
use std::fs
use std::time::{Duration, Instant};
use rand::Rng;
use std::thread::sleep;

fn read_file(filename) -> io::Result<Vec<String>> {
    let file_str = fs::read_to_string(filename)?;
    file_str.unwrap().lines().map(String::from).collect();
}

fn time_request(site) -> io::Result<Duration> {
    let mut rng = rand::thread_rng();
    let sleep_time = rng.gen_range(1..1000);
    let duration = Duration::from_millis(sleep_time);
    let start = Instant::now();
    sleep(duration);
    Ok(start.elapsed())
}

fn main -> io::Result<()> {
    // Esto se sacaría de la linea de comandos o donde fuere
    let n_threads = 4;
    let filename = "my_site_list.txt";

    // Setear el máximo de threads de Rayon
    let thread_pool_result = ThreadPoolBuilder::new()
        .num_threads(n_threads)
        .build_global();
    if let Err(e) = thread_pool_result {
        panic!("Failed to set number of threads: {}", e);
    }

    let sites = read_file(filename).unwrap();

    let total = sites
        .par_iter()
        .map(|site| time_request(site))
        .reduce(
            || Duration::new(0,0),
            |a: Duration, b: Duration|{a.checked_add(b).unwrap()})
        .unwrap();
    
    let avg = total.mul_f64(1.0/(sites.size() as f64))
    Ok(())
}

```

## Pregunta 3

### Enunciado

Implemente un Semaforo y sus métodos con pseudo codigo de Rust utilizando Condvars.

### Respuesta

```Rust
use std::sync::{Condvar, Mutex}

struct Semaphore {
    available: Condvar,
    _k: Mutex<u32>
}

impl Semaphore{
    pub fn new(k: u32) -> Semaphore{
        Semaphore{
            Condvar::new(),
            Mutex::new(k)
        }
    }

    pub fn wait(&self) -> () {
        let mut k = self._k.lock().unwrap();
        while *k == 0 {
            k = self.available.wait(k).unwrap();
        }
        *k -= 1;
    }

    pub fn signal(&self) -> () {
        let mut k = self._k.lock().unwrap();
        *k += 1;
        self.available.notify_one();
    }
}
```

---

#### 4. Realice una Red de Petri del problema Productor-Consumidor con buffer acotado.

---

