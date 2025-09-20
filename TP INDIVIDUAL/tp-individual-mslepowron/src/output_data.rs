//! Módulo para el manejo del resultado final del analisis realizado de las reviews.
use crate::review_error::ReviewError;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
///Estructura para almacenar las reviews procesadas, según los juegos e idiomas más popuares.
pub struct OutputData {
    padron: u32,
    top_games: Vec<Value>,
    top_languages: Vec<Value>,
}

impl OutputData {
    ///Crea una nueva instancia de OutputData
    pub fn new(top_games: Vec<Value>, top_languages: Vec<Value>) -> OutputData {
        OutputData {
            padron: 109454,
            top_games,
            top_languages,
        }
    }

    ///Almacena la informacion procesada que se encuentra en la estructura en un archivo .json.
    ///Retorna error si falla la serialización o la escritura del archivo.
    pub fn save_output_as_json(self, output_path: String) -> Result<(), ReviewError> {
        match serde_json::to_string_pretty(&self) {
            Ok(json) => match std::fs::write(output_path, json) {
                Ok(_) => Ok(()),
                Err(e) => Err(ReviewError::OutputWriteError(format!(
                    "Falló al escribir el archivo json de salida '{}'",
                    e
                ))),
            },
            Err(e) => Err(ReviewError::OutputWriteError(format!(
                "Falló la serializacion del archivo json de salida: {}",
                e
            ))),
        }
    }
}
