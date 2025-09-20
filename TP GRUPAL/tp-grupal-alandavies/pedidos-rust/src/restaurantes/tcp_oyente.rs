use crate::mensajes::eliminar_peido_en_curso::EliminarPedidoEnCurso;
use crate::mensajes::mcode::ELIMINAR_PEDIDO_EN_CURSO_CODE;
use crate::mensajes::{
    actualizar_siguiente::ActualizarSiguiente,
    agrega_pedido_en_curso::AgregarPedidoEnCurso,
    cambiar_siguiente::CambiarSiguiente,
    info_lider::InfoLider,
    info_restaurantes_interno::InfoRestaurantesInterno,
    keep_alive::KeepAlive,
    mcode::{
        ACTUALIZAR_SIGUIENTE_CODE, AGREGAR_PEDIDO_EN_CURSO_CODE, CAMBIAR_SIGUIENTE_CODE,
        INFO_LIDER_CODE, INFO_RESTAURANTES_INTERNO_CODE, KEEP_ALIVE_CODE, PEDIDO_TOMADO_CODE,
        PEDIR_CODE, PROGRESO_PEDIDO_CODE, RING_ELECTION_CODE, VER_RESTAURANTES_CODE,
    },
    pedido_tomado::PedidoTomado,
    pedir::Pedir,
    progreso_pedido::ProgresoPedido,
    ring_election::RingElection,
    ver_restaurantes::VerRestaurantes,
};
use crate::restaurantes::nodo_restaurante::NodoRestaurante;
use actix::Addr;
use std::net::SocketAddrV4;
use tokio::{io::AsyncReadExt, net::TcpListener};

pub struct TcpOyente {
    nodo_asociado: Addr<NodoRestaurante>,
    direccion: SocketAddrV4,
}

impl TcpOyente {
    pub fn new(nodo_asociado: Addr<NodoRestaurante>, direccion: SocketAddrV4) -> Self {
        TcpOyente {
            nodo_asociado,
            direccion,
        }
    }

    fn procesar_mensaje(&self, mcode: u8, data_buf: &[u8]) {
        match mcode {
            PROGRESO_PEDIDO_CODE => {
                let mensaje: Result<ProgresoPedido, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            PEDIDO_TOMADO_CODE => {
                let mensaje: Result<PedidoTomado, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            VER_RESTAURANTES_CODE => {
                let mensaje: Result<VerRestaurantes, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            PEDIR_CODE => {
                let mensaje: Result<Pedir, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            INFO_RESTAURANTES_INTERNO_CODE => {
                let mensaje: Result<InfoRestaurantesInterno, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            RING_ELECTION_CODE => {
                let mensaje: Result<RingElection, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            KEEP_ALIVE_CODE => {
                let mensaje: Result<KeepAlive, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            CAMBIAR_SIGUIENTE_CODE => {
                let mensaje: Result<CambiarSiguiente, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            INFO_LIDER_CODE => {
                let mensaje: Result<InfoLider, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            ACTUALIZAR_SIGUIENTE_CODE => {
                let mensaje: Result<ActualizarSiguiente, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            ELIMINAR_PEDIDO_EN_CURSO_CODE => {
                let mensaje: Result<EliminarPedidoEnCurso, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            AGREGAR_PEDIDO_EN_CURSO_CODE => {
                let mensaje: Result<AgregarPedidoEnCurso, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            _ => {}
        }
    }

    pub async fn start(&self) -> Result<(), std::io::Error> {
        println!("Intentando iniciar oyente TCP en: {}", self.direccion);

        match TcpListener::bind(self.direccion).await {
            Ok(listener) => {
                println!("✓ Oyente TCP iniciado exitosamente en: {}", self.direccion);

                loop {
                    match listener.accept().await {
                        Ok((mut stream, peer_addr)) => match self.leer_mensaje(&mut stream).await {
                            Ok((mcode, data_buf)) => {
                                self.procesar_mensaje(mcode, &data_buf);
                            }
                            Err(e) => {
                                eprintln!("Error leyendo mensaje de {}: {}", peer_addr, e);
                            }
                        },
                        Err(e) => {
                            eprintln!("Error aceptando conexión: {}", e);
                            continue;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("✗ Error al hacer bind en {}: {}", self.direccion, e);

                match e.kind() {
                    std::io::ErrorKind::AddrInUse => {
                        eprintln!("  → El puerto {} ya está en uso", self.direccion.port());
                        eprintln!("  → Sugerencias:");
                        eprintln!("    - Verificar si otro proceso está usando el puerto");
                        eprintln!(
                            "    - Usar 'netstat -tlnp | grep {}' para ver qué proceso lo usa",
                            self.direccion.port()
                        );
                        eprintln!("    - Cambiar a un puerto diferente");
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        eprintln!(
                            "  → Permisos insuficientes para hacer bind en {}",
                            self.direccion
                        );
                        eprintln!("  → Sugerencias:");
                        if self.direccion.port() < 1024 {
                            eprintln!("    - Los puertos < 1024 requieren privilegios de root");
                            eprintln!("    - Usar un puerto >= 1024 o ejecutar como root");
                        } else {
                            eprintln!("    - Verificar permisos del sistema");
                        }
                    }
                    std::io::ErrorKind::AddrNotAvailable => {
                        eprintln!(
                            "  → La dirección {} no está disponible",
                            self.direccion.ip()
                        );
                        eprintln!("  → Sugerencias:");
                        eprintln!("    - Verificar que la IP sea válida para esta máquina");
                        eprintln!("    - Usar 0.0.0.0 para escuchar en todas las interfaces");
                        eprintln!("    - Usar 127.0.0.1 para escuchar solo localmente");
                    }
                    _ => {
                        eprintln!("  → Error de tipo: {:?}", e.kind());
                    }
                }

                Err(e)
            }
        }
    }

    async fn leer_mensaje(
        &self,
        stream: &mut tokio::net::TcpStream,
    ) -> Result<(u8, Vec<u8>), std::io::Error> {
        let mut mcode_buf = [0u8; 1];
        stream.read_exact(&mut mcode_buf).await?;
        let mcode = mcode_buf[0];

        let mut length_buf = [0u8; 4];
        stream.read_exact(&mut length_buf).await?;
        let length = u32::from_be_bytes(length_buf) as usize;

        if length > 1_000_000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Mensaje demasiado grande: {} bytes", length),
            ));
        }

        let mut data_buf = vec![0u8; length];
        stream.read_exact(&mut data_buf).await?;

        Ok((mcode, data_buf))
    }

    pub async fn verificar_puerto_disponible(&self) -> bool {
        TcpListener::bind(self.direccion).await.is_ok()
    }
}
