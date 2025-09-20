//! Módulo para almacenar todos los datos de una reseña particular.
//! Permite unificar diferentes instancias de reseñas que aplican a un mismo juego para recopilar la
//! información de todo el archivo.
use crate::game::Game;
use crate::language::Language;
use crate::output_data::OutputData;
use serde_json::Value;
use std::collections::HashMap;

///Estructura que representa los resultados agregados de reseñas, organizados por juego y por idioma.
pub struct ReviewResult {
    game: HashMap<String, Game>,
    language: HashMap<String, Language>,
}

impl Default for ReviewResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ReviewResult {
    ///Creación de una nueva instancia de ReviewResult
    pub fn new() -> ReviewResult {
        ReviewResult {
            game: HashMap::new(),
            language: HashMap::new(),
        }
    }

    ///Permite almacenar en la estructura los datos de una reseña particular.
    pub fn load_review_data(
        game: HashMap<String, Game>,
        language: HashMap<String, Language>,
    ) -> ReviewResult {
        ReviewResult { game, language }
    }

    ///Fusiona dos ReviewResult combinando estadísticas de juegos e idiomas.
    /// Devuelve un ReviewResult con los datos acumulados.
    pub fn reduce(mut self, other: &ReviewResult) -> ReviewResult {
        self = self.reduce_game_stats(other);
        self = self.reduce_language_stats(other);
        self
    }

    ///Unifica los datos del Game de la ReviewResult con el game de otra.
    /// Si el Game ya está registrado, se actualizan sus valores. Si no está registrado, se crea con los datos contenidos en 'other'
    fn reduce_game_stats(mut self, other: &ReviewResult) -> ReviewResult {
        for (game_name, other_game) in &other.game {
            self.game
                .entry(game_name.clone())
                .and_modify(|current_game| current_game.merge_game_wiith_other_review(other_game))
                .or_insert_with(|| other_game.clone());
        }
        self
    }

    ///Unifica los datos del Language de la ReviewResult con el language de otra reseña.
    /// Si el Language ya está registrado, se actualizan sus valores. Si no está registrado, se crea con los datos contenidos en 'other'
    fn reduce_language_stats(mut self, other: &ReviewResult) -> ReviewResult {
        for (language, other_language_data) in &other.language {
            self.language
                .entry(language.clone())
                .and_modify(|language_data| {
                    language_data.merge_language_with_other_review(other_language_data)
                })
                .or_insert_with(|| other_language_data.clone());
        }
        self
    }

    ///Devuelve los top 'n' juegos e idiomas más populares, dada una cantidad solicitada de cada uno.
    pub fn get_top_results(
        self,
        num_games_to_display: usize,
        num_languages_to_display: usize,
    ) -> OutputData {
        let top_games: Vec<Value> = Game::top_n_games_reviewed(self.game, num_games_to_display);
        let top_languages: Vec<Value> =
            Language::top_n_languages(self.language, num_languages_to_display);

        OutputData::new(top_games, top_languages)
    }
}
