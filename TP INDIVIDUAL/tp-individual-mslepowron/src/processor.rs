//! Modulo que se encarga del procesaminto de la informacion comprendida en el/los csv, realizando este
//! proceso de manera concurrente, siguiendo un modelo fork-join
use crate::review_error::ReviewError;
use crate::review_record::ReviewRecord;
use crate::review_result::ReviewResult;
use csv::Reader;
use rayon::prelude::*;
use std::fs::{read_dir, File};
use std::path::PathBuf;

/// Procesa multiples archivos CSV con resenas que siguen el formato de steam_reviews.csv del dataset de kaggle: https://www.kaggle.com/datasets/najzeko/steam-reviews-2021
/// Guarda los resultados consolidados de las renas analizadas en un archivo JSON de salida.
/// Devuelve Error en caso de que no se pueda leer el directorio provisto o halla algun error al leer los files del directorio.
pub fn fork_join(threads: usize, dir_path: String, output_path: String) -> Result<(), ReviewError> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .map_err(|e| ReviewError::ThreadPoolBuildError(e.to_string()))?;

    match read_dir(&dir_path) {
        Ok(dir_entries) => {
            let result = dir_entries
                .flatten()
                .map(|d| d.path())
                .collect::<Vec<PathBuf>>()
                .par_iter()
                .flat_map(|path| {
                    let file = File::open(path);

                    let reader = Reader::from_reader(file.unwrap());
                    reader.into_deserialize::<ReviewRecord>().par_bridge()
                })
                .map(|res| match res {
                    Ok(record) => record.process_review(),
                    Err(_e) => ReviewResult::new(),
                })
                .reduce(ReviewResult::new, |mut acc, map| {
                    acc = acc.reduce(&map);
                    acc
                });

            let output_data = result.get_top_results(3, 3);
            output_data.save_output_as_json(output_path)?;

            Ok(())
        }
        Err(_e) => Err(ReviewError::DirectoryNotFound(
            "Error al leer el directorio.".to_string(),
        )),
    }
}
