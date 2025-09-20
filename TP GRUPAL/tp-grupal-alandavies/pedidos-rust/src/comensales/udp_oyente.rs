use crate::{
    comensales::nodo_comensal::NodoComensal,
    mensajes::{
        info_restaurantes::InfoRestaurantes,
        llego_pedido::LlegoPedido,
        mcode::{
            INFO_RESTAURANTES_CODE, LLEGO_PEDIDO_CODE, PAGO_AUTORIZADO_CODE, PAGO_RECHAZADO_CODE,
            PROGRESO_PEDIDO_CODE,
        },
        pago_autorizado::PagoAutorizado,
        pago_rechazado::PagoRechazado,
        progreso_pedido::ProgresoPedido,
    },
};
use actix::Addr;
use std::net::SocketAddrV4;
use tokio::net::UdpSocket;

/// Oyente UDP que escucha mensajes de otros nodos y los procesa.
/// Este oyente se asocia a un `NodoComensal` y recibe mensajes relacionados con pedidos, pagos y restaurantes.
///
/// Campos:
/// - `nodo_asociado`: Dirección del nodo comensal asociado que recibirá los mensajes.
/// - `direccion`: Dirección UDP en la que el oyente escucha los mensajes.
pub struct UdpOyente {
    nodo_asociado: Addr<NodoComensal>,
    direccion: SocketAddrV4,
}

impl UdpOyente {
    pub fn new(nodo_asociado: Addr<NodoComensal>, direccion: SocketAddrV4) -> Self {
        UdpOyente {
            nodo_asociado,
            direccion,
        }
    }

    /// Procesa el mensaje recibido según su código (mcode).
    fn procesar_mensaje(&self, mcode: u8, data_buf: &[u8]) {
        match mcode {
            INFO_RESTAURANTES_CODE => {
                let mensaje: Result<InfoRestaurantes, serde_json::Error> =
                    serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    println!("Error al procesar InfoRestaurantes: {:?}", mensaje);
                }
            }
            PROGRESO_PEDIDO_CODE => {
                let mensaje: Result<ProgresoPedido, serde_json::Error> =
                    serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    println!("Error al procesar ProgresoPedido: {:?}", mensaje);
                }
            }
            PAGO_RECHAZADO_CODE => {
                let mensaje: Result<PagoRechazado, serde_json::Error> =
                    serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    println!("Error al procesar PagoRechazado: {:?}", mensaje);
                }
            }
            PAGO_AUTORIZADO_CODE => {
                let mensaje: Result<PagoAutorizado, serde_json::Error> =
                    serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    println!("Error al procesar PagoAutorizado: {:?}", mensaje);
                }
            }
            LLEGO_PEDIDO_CODE => {
                let mensaje: Result<LlegoPedido, serde_json::Error> =
                    serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                } else {
                    println!("Error al procesar LlegoPedido: {:?}", mensaje);
                }
            }
            _ => {}
        }
    }

    /// Inicia el oyente UDP y procesa los mensajes entrantes.
    pub async fn start(&self) -> Result<(), std::io::Error> {
        let socket = UdpSocket::bind(self.direccion).await?;
        println!("Escuchando UDP en {}", self.direccion);
        let mut buf = [0u8; 2048];

        loop {
            let (len, _peer_addr) = socket.recv_from(&mut buf).await?;
            if len < 9 {
                // Debe tener al menos 1 byte de mcode y 8 de longitud (usize)
                continue;
            }

            let mcode = buf[0];
            let length_buf = [
                buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
            ];
            let length = usize::from_be_bytes(length_buf);

            if len < 9 + length {
                // Le faltan datos para completar el mensaje
                continue;
            }

            let data_buf = &buf[9..9 + length];
            self.procesar_mensaje(mcode, data_buf);
        }
    }
}
