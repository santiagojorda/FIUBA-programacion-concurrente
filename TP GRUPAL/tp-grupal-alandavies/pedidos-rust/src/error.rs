use std::fmt;

use crate::{
    app_config::error::ConfigError, args_parser::error::ArgsError, logger::log::LoggerError,
};

#[derive(Debug)]
pub enum Error {
    Config(ConfigError),
    Args(ArgsError),
    Io(std::io::Error),
    Logger(LoggerError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Args(e) => write!(f, "{}", e),
            Self::Config(e) => write!(f, "{}", e),
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Logger(e) => write!(f, "Logger error: {}", e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl std::error::Error for Error {}

impl From<ConfigError> for Error {
    fn from(e: ConfigError) -> Self {
        Error::Config(e)
    }
}

impl From<ArgsError> for Error {
    fn from(e: ArgsError) -> Self {
        Error::Args(e)
    }
}

impl From<LoggerError> for Error {
    fn from(e: LoggerError) -> Self {
        Error::Logger(e)
    }
}
