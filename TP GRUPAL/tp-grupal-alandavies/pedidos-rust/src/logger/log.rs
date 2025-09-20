use chrono::Utc;
use colored::{Color, Colorize};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
enum LogLevel {
    Info(Color),
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub enum AppType {
    Repartidor,
    NodoRepartidor,
    Restaurante,
    NodoRestaurante,
    Comensal,
    NodoComensal,
    Pagos,
}

#[derive(Debug, Clone)]
pub struct Logger {
    log_file: PathBuf,
    standard_output: bool,
    app_type: AppType,
    port: u16,
}

impl Logger {
    /// Creates a new `Logger` instance.
    ///
    /// # Parameters
    /// - `log_dir`: Path to the directory where the log file should be created.
    /// - `app_type`: The type of application for naming the log file.
    /// - `port`: The port number to include in the log file name.
    /// - `standard_output`: Whether to output logs to console as well.
    ///
    /// # Returns
    /// A new `Logger` instance.
    pub fn new(
        log_dir: &Path,
        app_type: AppType,
        port: u16,
        standard_output: bool,
    ) -> Result<Self, LoggerError> {
        if !log_dir.exists() {
            std::fs::create_dir_all(log_dir).map_err(LoggerError::from)?;
        } else if !log_dir.is_dir() {
            return Err(LoggerError::InvalidPath(
                "Provided path is not a directory.".into(),
            ));
        }

        let prefix = match app_type {
            AppType::Repartidor => "repartidor",
            AppType::NodoRepartidor => "nodo_repartidor",
            AppType::Restaurante => "restaurante",
            AppType::NodoRestaurante => "nodo_restaurante",
            AppType::Comensal => "comensal",
            AppType::NodoComensal => "nodo_comensal",
            AppType::Pagos => "pagos",
        };

        let log_file = log_dir.join(format!("{prefix}_{port}.log"));

        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_file)
            .map_err(LoggerError::from)?;

        Ok(Logger {
            log_file,
            standard_output,
            app_type,
            port,
        })
    }

    // Generic method for writing log messages
    fn log(&self, level: LogLevel, message: &str) -> Result<(), LoggerError> {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let prefix_log_message = match self.app_type {
            AppType::Repartidor => format!("[REPARTIDOR {}]", self.port),
            AppType::NodoRepartidor => format!("[NODO_REPARTIDOR {}]", self.port),
            AppType::Restaurante => format!("[RESTAURANTE {}]", self.port),
            AppType::NodoRestaurante => format!("[NODO_RESTAURANTE {}]", self.port),
            AppType::Comensal => format!("[COMENSAL {}]", self.port),
            AppType::NodoComensal => format!("[NODO_COMENSAL {}]", self.port),
            AppType::Pagos => "[PAGOS]".to_string(),
        };
        let log_message = match &level {
            LogLevel::Info(_) => format!(
                "{} [INFO] [{}]: {}\n",
                prefix_log_message, timestamp, message
            ),
            LogLevel::Warn => format!(
                "{} [WARN] [{}]: {}\n",
                prefix_log_message, timestamp, message
            ),
            LogLevel::Error => format!(
                "{} [ERROR] [{}]: {}\n",
                prefix_log_message, timestamp, message
            ),
        };

        // If logging to console, apply colors
        if self.standard_output {
            let colored_message = match &level {
                LogLevel::Info(color) => format!("{}", log_message.color(*color)),
                LogLevel::Warn => format!("{}", log_message.yellow()),
                LogLevel::Error => format!("{}", log_message.red()),
            };
            print!("{}", colored_message);
            io::stdout().flush().map_err(LoggerError::from)?;
        }

        // Open the file, write the log message, and close the file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .map_err(LoggerError::from)?;
        file.write_all(log_message.as_bytes())
            .map_err(LoggerError::from)?;
        file.flush().map_err(LoggerError::from)?;

        Ok(())
    }

    /// Logs an informational message.
    ///
    /// # Parameters
    /// - `message`: The informational message to log.
    /// - `color`: The color to use for the console output.
    pub fn info(&self, message: &str, color: Color) -> Result<(), LoggerError> {
        self.log(LogLevel::Info(color), message)
    }

    /// Logs a warning message.
    ///
    /// # Parameters
    /// - `message`: The warning message to log.
    pub fn warn(&self, message: &str) -> Result<(), LoggerError> {
        self.log(LogLevel::Warn, message)
    }

    /// Logs an error message.
    ///
    /// # Parameters
    /// - `message`: The error message to log.
    pub fn error(&self, message: &str) -> Result<(), LoggerError> {
        self.log(LogLevel::Error, message)
    }
}

#[derive(Debug)]
pub enum LoggerError {
    IoError(std::io::Error),
    InvalidPath(String),
}

impl std::fmt::Display for LoggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggerError::IoError(e) => write!(f, "I/O Error: {}", e),
            LoggerError::InvalidPath(msg) => write!(f, "Invalid Path: {}", msg),
        }
    }
}

impl std::error::Error for LoggerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoggerError::IoError(e) => Some(e),
            LoggerError::InvalidPath(_) => None,
        }
    }
}

impl From<std::io::Error> for LoggerError {
    fn from(err: std::io::Error) -> Self {
        LoggerError::IoError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_logger_creation_and_logging() {
        let log_dir = Path::new("/tmp/test_logs");
        fs::create_dir_all(log_dir).expect("Failed to create test directory");

        let logger =
            Logger::new(log_dir, AppType::Comensal, 8080, false).expect("Failed to create logger");

        let message = "Test log message.";
        logger
            .info(message, Color::Green)
            .expect("Failed to log message");

        let log_file_path = log_dir.join("comensal_8080.log");
        let log_contents = fs::read_to_string(&log_file_path).expect("Failed to read log file");

        assert!(log_contents.contains("[INFO]"), "INFO level missing in log");
        assert!(log_contents.contains(message), "Logged message missing");

        fs::remove_dir_all(log_dir).expect("Failed to remove test directory");
    }

    #[test]
    fn test_invalid_path() {
        let invalid_path = Path::new("/invalid/path");

        let result = Logger::new(invalid_path, AppType::Comensal, 8080, false);
        assert!(result.is_err(), "Logger should fail with an invalid path");
    }
}
