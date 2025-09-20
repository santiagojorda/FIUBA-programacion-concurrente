#  Enunciado evaluacion parcial 2C 2023 

---

#### 1. Determine y justifique cuál modelo de concurrencia convendría más utilizar para solucionar cada uno de los siguientes problemas:

- Cálculo de multiplicacion entre matrices para entrenar redes neuronales.
- Programa que realiza multiples requests a APIs y al finalizar combina los resultados.
- Procesamiento de logs de acceso de un sitio web muy concurrido.
- Backend de un juego en linea.


(***Nota post-parcial:*** *Las ultimas dos son medio ambiguas, pero creo que apuntaban a:*
- *Procesamiento de varios archivos con logs, datos independientes.*
- *Conexiones establecidas entre jugadores y estado compartido de recursos*)

---

#### 2. Desarrolle pseudo codigo de Rust para solucionar el siguiente problema:

Se quiere medir una aproximacion del tiempo de latencia que experimenta un servicio de conexión a internet.
Para ello se requiere desarrollar un programa que realiza peticiones concurrentemente a distintos sitios web que se obtienen como lineas a partir de un archivo (El archivo tiene 100 lineas).
El programa deberá realizar una peticion por sitio, de manera concurrente, y calcular el tiempo que
tarda la peticion en ser respondida. Al finalizar se debe promediar el tiempo de demora de todos los sitios.
Para modelar las request utilice `thread::sleep` con una duración random en segundos.
Tener en cuenta que si bien se quiere optimizar las peticiones de forma concurrente, se debe tener un
limite de N peticiones para no saturar la red y para limitar el numero de threads.

Realizar el DAG del problema

(***Nota post-parcial:*** *Algo que podia confundirse era si el problema requeria solucionarse con async o con map-reduce utilizando rayon. Creo que la idea apuntaba a la segunda, justamente para poder realizar el DAG.*)

---

#### 3. Implemente un Semaforo y sus métodos con pseudo codigo de Rust utilizando Condvars.

---

#### 4. Realice una Red de Petri del problema Productor-Consumidor con buffer acotado.

---

