//! Módulo para manejar los datos estadísticos sobre la reseña de un juego

use crate::language::Language;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

///Estructura para almacenar los datos de un Juego que obtuvo una reseña.
/// Se contabiliza la cantidad de reseñas que se hicieron para un Juego en particular y
/// se almacenan los idiomas en los que se realizó la review de un Juego en particular
#[derive(Clone, Serialize)]
pub struct Game {
    reviews: u32,
    languages: HashMap<String, Language>,
}

impl Game {
    ///Funcion para crear una nueva instancia de Game
    pub fn new(reviews: u32, languages: HashMap<String, Language>) -> Game {
        Game { reviews, languages }
    }

    ///Funcion para unificar la informacion de la reseña de un Game con otro, si son el mismo juego segun el nombre
    /// Se aumenta la cantidad de reviews hechas para ese juego y se unifica la informacion de los idiomas
    /// utilizados para hacer reseñas de ese Game
    pub fn merge_game_wiith_other_review(&mut self, other: &Game) {
        self.reviews += other.reviews;

        for (language, other_lang_data) in &other.languages {
            self.languages
                .entry(language.clone())
                .and_modify(|lang_info| {
                    lang_info.review_count += other_lang_data.review_count;
                    lang_info
                        .top_reviews
                        .extend(other_lang_data.top_reviews.clone());

                    if let Some(max_review) = lang_info
                        .top_reviews
                        .iter()
                        .max_by_key(|r| r.votes_helpful)
                        .cloned()
                    {
                        lang_info.top_reviews = vec![max_review];
                    }
                })
                .or_insert_with(|| other_lang_data.clone());
        }
    }

    ///Devuelve los top 'n' juegos más populares, dada una cantidad solicitada, junto con los top 3 idiomas utilizados
    /// para realizar reseñas de ese juego.
    pub fn top_n_games_reviewed(
        games: HashMap<String, Game>,
        num_games_to_display: usize,
    ) -> Vec<Value> {
        let mut top_games: Vec<_> = games.into_iter().collect();
        let mut top_games_output: Vec<Value> = Vec::new();

        top_games.sort_by(|a, b| b.1.reviews.cmp(&a.1.reviews).then_with(|| a.0.cmp(&b.0)));
        top_games.truncate(num_games_to_display);

        for (game_name, game) in &mut top_games {
            let mut top_game_langs: Vec<_> = game.languages.iter().collect();
            top_game_langs.sort_by(|a, b| {
                b.1.review_count
                    .cmp(&a.1.review_count)
                    .then_with(|| a.0.cmp(b.0))
            });
            top_game_langs.truncate(3);

            let mut game_languages_output = Vec::new();

            for (language, language_data) in top_game_langs {
                if let Some(top_review) = language_data.top_reviews.first() {
                    let language_entry = json!({
                        "language": language,
                        "review_count": language_data.review_count,
                        "top_review": top_review.text.clone(),
                        "top_review_votes": top_review.votes_helpful,
                    });
                    game_languages_output.push(language_entry);
                }
            }
            let game_entry = json!({
                "game": game_name,
                "review_count": game.reviews,
                "languages": game_languages_output,
            });
            top_games_output.push(game_entry);
        }

        top_games_output
    }
}
