use crate::mensajes::{
    actualizar_siguiente::ActualizarSiguiente,
    cambiar_siguiente::CambiarSiguiente,
    cobrar::Cobrar,
    info_pedido_en_curso::InfoPedidoEnCurso,
    info_repartidor_pedido::InfoRepartidorPedido,
    keep_alive::KeepAlive,
    mcode::{
        ACTUALIZAR_SIGUIENTE_CODE, CAMBIAR_SIGUIENTE_CODE, COBRAR_CODE, INFO_PEDIDO_EN_CURSO_CODE,
        INFO_REPARTIDOR_PEDIDO_CODE, KEEP_ALIVE_CODE, OFRECER_PEDIDO_CODE,
        PEDIDOS_EN_CURSO_REPARTIDORES_CODE, PEDIDO_LISTO_CODE, QUITAR_PEDIDO_DE_BUFFER_CODE,
        RING_ELECTION_CODE, SOLICITAR_PEDIDO_EN_CURSO_CODE,
    },
    ofrecer_pedido::OfrecerPedido,
    pedido_listo::PedidoListo,
    pedidos_en_curso_repartidores::PedidosEnCursoRepartidores,
    quitar_pedido_de_buffer::QuitarPedidoDeBuffer,
    ring_election::RingElection,
    solicitar_pedido_en_curso::SolicitarPedidoEnCurso,
};
use crate::repartidores::nodo_repartidor::NodoRepartidor;
use actix::Addr;
use std::net::SocketAddrV4;
use tokio::{io::AsyncReadExt, net::TcpListener};

pub struct TcpOyente {
    nodo_asociado: Addr<NodoRepartidor>,
    direccion: SocketAddrV4,
}

impl TcpOyente {
    pub fn new(nodo_asociado: Addr<NodoRepartidor>, direccion: SocketAddrV4) -> Self {
        TcpOyente {
            nodo_asociado,
            direccion,
        }
    }

    fn procesar_mensaje(&self, mcode: u8, data_buf: &[u8]) {
        match mcode {
            COBRAR_CODE => {
                let mensaje: Result<Cobrar, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando Cobrar");
                }
            }
            OFRECER_PEDIDO_CODE => {
                let mensaje: Result<OfrecerPedido, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando OfrecerPedido");
                }
            }
            PEDIDO_LISTO_CODE => {
                let mensaje: Result<PedidoListo, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando PedidoListo");
                }
            }
            RING_ELECTION_CODE => {
                let mensaje: Result<RingElection, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando RingElection");
                }
            }
            KEEP_ALIVE_CODE => {
                let mensaje: Result<KeepAlive, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando KeepAlive");
                }
            }
            CAMBIAR_SIGUIENTE_CODE => {
                let mensaje: Result<CambiarSiguiente, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando CambiarSiguiente");
                }
            }
            ACTUALIZAR_SIGUIENTE_CODE => {
                let mensaje: Result<ActualizarSiguiente, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando ActualizarSiguiente");
                }
            }
            SOLICITAR_PEDIDO_EN_CURSO_CODE => {
                let mensaje: Result<SolicitarPedidoEnCurso, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando SolicitarPedidoEnCurso");
                }
            }
            INFO_REPARTIDOR_PEDIDO_CODE => {
                let mensaje: Result<InfoRepartidorPedido, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando InfoRepartidorPedido");
                }
            }
            INFO_PEDIDO_EN_CURSO_CODE => {
                let mensaje: Result<InfoPedidoEnCurso, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando InfoPedidoEnCurso");
                }
            }
            PEDIDOS_EN_CURSO_REPARTIDORES_CODE => {
                let mensaje: Result<PedidosEnCursoRepartidores, _> =
                    serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando PedidosEnCursoRepartidores");
                }
            }
            QUITAR_PEDIDO_DE_BUFFER_CODE => {
                let mensaje: Result<QuitarPedidoDeBuffer, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    eprintln!("Error deserializando QuitarPedidoDeBuffer");
                }
            }
            _ => {
                eprintln!("Código de mensaje desconocido: {}", mcode);
            }
        }
    }

    pub async fn start(&self) -> Result<(), std::io::Error> {
        match TcpListener::bind(self.direccion).await {
            Ok(listener) => loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    if let Ok((mcode, data_buf)) = self.leer_mensaje(&mut stream).await {
                        self.procesar_mensaje(mcode, &data_buf);
                    }
                }
            },

            Err(e) => Err(e),
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
