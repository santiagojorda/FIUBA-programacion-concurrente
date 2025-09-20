//! Módulo para almacenar los datos de cada record del csv (cada review) que son necesarios para calcular el output del programa
use crate::game::Game;
use crate::language::Language;
use crate::review_result::ReviewResult;
use crate::top_review::TopReview;
use serde::Deserialize;
use std::collections::HashMap;

/// Estructura que representa los datos de una review del/los CSV original que se recibe como input.
/// Se almacenan unicamente los datos mínimos e indispensables para poder procesar las reviews, el resto se descartan.
#[derive(Debug, Deserialize)]
pub struct ReviewRecord {
    app_name: String,
    language: String,
    review: String,
    votes_helpful: u32,
}

impl ReviewRecord {
    ///Procesa los datos del CSV almacenados en la misma, y transforma la review cruda en una estructura
    /// ReviewResult para poder calcular las estadísticas necesarias.
    pub fn process_review(self) -> ReviewResult {
        let mut game_map = HashMap::new();
        let mut language_map = HashMap::new();

        let top_review = TopReview::new(self.review.clone(), self.votes_helpful);

        // datos del idioma de la review para desp levantar en la seccion de languages
        let language = Language::new(1, vec![top_review.clone()]);
        language_map.insert(self.language.clone(), language);

        // datos del juego; incluye en language del juego xq asi desp puedo calcular los top idiomas de resenas de ese juego y demas.
        let game_language = Language::new(1, vec![top_review]);
        let mut game_language_map = HashMap::new();
        game_language_map.insert(self.language, game_language);

        let game = Game::new(1, game_language_map);
        game_map.insert(self.app_name, game);

        ReviewResult::load_review_data(game_map, language_map)
    }
}
