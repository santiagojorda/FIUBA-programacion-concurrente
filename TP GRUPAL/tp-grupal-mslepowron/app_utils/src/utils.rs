use crate::constants::SERVER_TCP_PORT_RANGE;
use log::{error, info};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::io;
use thiserror::Error;
use tokio::net::TcpStream;

#[derive(Serialize)]
pub struct SerializableMessage {
    pub title: String,
    pub payload: serde_json::Value,
}

#[derive(Error, Debug)]
pub enum DeserializationError {
    #[error("El mensaje no contiene el campo especificado {0}")]
    MissingField(String),
    #[error("Error al deserializar el mensaje en el campo {0}")]
    InvalidFormat(String),
}

pub async fn connect_to_server() -> io::Result<TcpStream> {
    // En lugar de la constante, usa id_to_tcp_address(1):
    let addr = id_to_tcp_address(1);
    let stream = TcpStream::connect(&addr).await;
    match stream {
        Ok(conn) => {
            info!("Conectado al servidor en {}", addr);
            Ok(conn)
        }
        Err(e) => {
            error!("Error al conectar al servidor {}: {}", addr, e);
            Err(e)
        }
    }
}

/// Se utiliza para serializar el mensaje que se envia a traves de un TcpMessage
pub fn serialize_message<T: Serialize>(title: &str, message: T) -> String {
    let Ok(payload) = serde_json::to_value(&message) else {
        error!("No fue posible serializar el mensaje {}", title);
        return String::default();
    };
    serde_json::to_string(&SerializableMessage {
        title: title.into(),
        payload,
    })
    .expect("Se deberia poder serializar el mensaje")
}

///Se deserializa un String que viaja en el TcpMessage por un TcpSender. Se obtiene un titulo de mensaje que lo representa y un payload con datos de, por ejemplo, un cliente.
pub fn deserialize_tcp_message(msg_str: &str) -> (String, Value) {
    let Ok(message): Result<Value, serde_json::Error> = serde_json::from_str(msg_str) else {
        return (String::default(), Value::default());
    };

    let Some(name) = message["title"].as_str() else {
        return (String::default(), Value::default());
    };

    let Some(payload) = message.get("payload") else {
        return (String::default(), Value::default());
    };
    (name.to_owned(), payload.to_owned())
}

pub fn deserialize_payload<T: DeserializeOwned>(
    message: &Value,
    field: &str,
) -> Result<T, DeserializationError> {
    let Some(payload) = message.get(field) else {
        return Err(DeserializationError::MissingField(field.to_string()));
    };
    let deserialized: T = serde_json::from_value(payload.clone())
        .map_err(|_| DeserializationError::InvalidFormat(field.to_string()))?;
    Ok(deserialized)
}

/// Recibe un String en formato json que representa un mensaje.
/// Separa el mensaje en el nombre del mensaje, y el payload del mensaje.
/// Si falla al deserializar, devuelve un mensaje con nombre string vacio.
pub fn split_message(message_string: &str) -> (String, Value) {
    // convierto el string json a un Value
    let Ok(message): Result<Value, serde_json::Error> = serde_json::from_str(message_string) else {
        return (String::default(), Value::default());
    };

    // del Value extraigo el name y lo convierto a String
    let Some(name) = message["name"].as_str() else {
        return (String::default(), Value::default());
    };

    // del Value extraigo el payload
    let Some(payload) = message.get("payload") else {
        return (String::default(), Value::default());
    };
    (name.to_owned(), payload.to_owned())
}

// Función auxiliar para calcular distancia euclídea al cuadrado
pub fn distance_squared(a: (u64, u64), b: (u64, u64)) -> u64 {
    let dx = a.0 as i64 - b.0 as i64;
    let dy = a.1 as i64 - b.1 as i64;
    (dx * dx + dy * dy) as u64
}

/// Dado un numero de id, devuelvo el puerto del servidor donde se deberia encontrar el servidor con dicho id
pub fn id_to_tcp_address(id: u64) -> String {
    let tcp_port = SERVER_TCP_PORT_RANGE + id;
    format!("127.0.0.1:{tcp_port}")
}
