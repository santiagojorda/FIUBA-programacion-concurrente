#[derive(Debug)]
pub enum ReviewError {
    DirectoryNotFound(String),
    //FileOpenError(String),
    //CsvReadError(String),
    OutputWriteError(String),
    ThreadPoolBuildError(String),
}

impl ReviewError {
    pub fn display_error(self) {
        match self {
            ReviewError::DirectoryNotFound(p) => eprintln!("No se encontró el directorio: {}", p),
            //ReviewError::FileOpenError(p) => eprintln!("No se pudo abrir el archivo: {}", p),
            //ReviewError::CsvReadError(msg) => eprintln!("Error al leer CSV: {}", msg),
            ReviewError::OutputWriteError(p) => {
                eprintln!("No se pudo escribir el archivo de salida: {}", p)
            }
            ReviewError::ThreadPoolBuildError(e) => {
                eprintln!("Error al construir el pool de hilos: {}", e)
            }
        }
    }
}
