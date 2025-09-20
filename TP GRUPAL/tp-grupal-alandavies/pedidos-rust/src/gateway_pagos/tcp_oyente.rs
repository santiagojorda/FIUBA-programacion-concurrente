use crate::{
    gateway_pagos::pagos::GatewayPagos,
    mensajes::{
        cobrar::Cobrar,
        intento_pago::IntentoPago,
        mcode::{COBRAR_CODE, INTENTO_PAGO_CODE},
    },
};
use actix::Addr;
use std::net::SocketAddrV4;
use tokio::{io::AsyncReadExt, net::TcpListener};

/// Escucha conexiones TCP y procesa mensajes de pago.
pub struct TcpOyente {
    nodo_asociado: Addr<GatewayPagos>,
    direccion: SocketAddrV4,
}

impl TcpOyente {
    pub fn new(nodo_asociado: Addr<GatewayPagos>, direccion: SocketAddrV4) -> Self {
        TcpOyente {
            nodo_asociado,
            direccion,
        }
    }

    /// Procesa el mensaje recibido según su código.
    fn procesar_mensaje(&self, mcode: u8, data_buf: &[u8]) {
        match mcode {
            INTENTO_PAGO_CODE => {
                let mensaje: Result<IntentoPago, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            COBRAR_CODE => {
                let mensaje: Result<Cobrar, _> = serde_json::from_slice(data_buf);
                if let Ok(mensaje) = mensaje {
                    self.nodo_asociado.do_send(mensaje);
                }
            }
            _ => {}
        }
    }

    /// Inicia el oyente TCP, aceptando conexiones y procesando mensajes.
    pub async fn start(&self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(self.direccion).await?;

        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let mut mcode_buf = vec![0u8; 1];
                    let mut length_buf = vec![0u8; 4];
                    let _ = stream.read(&mut mcode_buf).await;
                    let _ = stream.read(&mut length_buf).await;
                    let length =
                        u32::from_be_bytes(length_buf.try_into().unwrap_or([0u8; 4])) as usize;
                    let mut data_buf = vec![0u8; length];
                    let _ = stream.read(&mut data_buf).await;
                    let mcode = mcode_buf[0];
                    self.procesar_mensaje(mcode, &data_buf);
                }
                Err(e) => {
                    println!("Error aceptando conexión: {}", e);
                }
            };
        }
    }
}
