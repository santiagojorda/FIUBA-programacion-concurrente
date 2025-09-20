use std::{fmt, num::ParseIntError};

pub const ARGUMENTOS_INVALIDOS: &str = "Argumentos inválidos. Formato: cargo run --bin <nombre_binario> <path_config> <puerto> <x> <y>";
pub const ARGUMENTOS_INVALIDOS_PAGOS: &str =
    "Argumentos inválidos. Formato: cargo run --bin pagos <path_config>";
pub const PATH_INVALIDO: &str = "No se pudo encontrar el archivo de configuración ubicado en ";
pub const PUERTO_RANGO_INVALIDO: &str =
    "El puerto debe estar dentro del rango de puertos de repartidores";

#[derive(Debug)]
pub enum ArgsError {
    InvalidArgs(String),
    InvalidConfigPath(String),
    InvalidCoord(ParseIntError),
    InvalidPort(String),
}

impl fmt::Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgs(e) => write!(f, "{e}"),
            Self::InvalidConfigPath(e) => write!(f, "{e}"),
            Self::InvalidCoord(e) => write!(f, "Coordenada inválida: {}", e),
            Self::InvalidPort(e) => write!(f, "Puerto inválido: {}", e),
        }
    }
}

impl std::error::Error for ArgsError {}
