use std::fmt;

pub const ERROR_RANGO_PUERTOS_REPARTIDORES: &str = "Rango de puertos inválido para repartidores";
pub const ERROR_RANGO_PUERTOS_RESTAURANTES: &str = "Rango de puertos inválido para restaurantes";
pub const ERROR_IPS_DUPLICADAS: &str = "Todas las aplicaciones deben tener una IP distinta";
pub const ERROR_LOGGER_DIR_INVALIDO: &str = "Directorio del logger inválido";

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(toml::de::Error),
    ValidationError(String),
}

impl ConfigError {
    pub fn validation_error(message: &str) -> Self {
        ConfigError::ValidationError(message.to_string())
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
            ConfigError::ValidationError(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::ParseError(err)
    }
}
