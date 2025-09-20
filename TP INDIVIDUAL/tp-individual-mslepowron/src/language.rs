//! Módulo para manejar los datos estadísticos sobre los idiomas en los que se realizan las reseñas.
use std::collections::HashMap;

use crate::top_review::TopReview;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Serialize)]
///Estructura para almacenar los datos de un idioma en el que se escribió una reseña.
/// ###  review_count
/// Se contabiliza la cantidad de reseñas que se escribieron en un idioma en particular
/// ###  top_reviews
/// Se almacenan las reseñas más votadas por usuarios que fueron escritas en un idioma en particualr
pub struct Language {
    pub review_count: u32,
    pub top_reviews: Vec<TopReview>,
}

impl Language {
    ///Funcion para crear una nueva instancia de Language
    pub fn new(review_count: u32, review_in_language: Vec<TopReview>) -> Language {
        Language {
            review_count,
            top_reviews: review_in_language,
        }
    }

    ///Funcion para unificar la informacion del Language de una reseña con otro, si son el mismo idioma segun su nombre
    /// Se aumenta la cantidad de reviews hechas en ese idioma y se unifica la informacion de las reseñas mas populares
    /// votadas por los usuarios que fueron escritas en ese idoma.
    pub fn merge_language_with_other_review(&mut self, other: &Language) {
        self.review_count += other.review_count;
        self.top_reviews.extend(other.top_reviews.clone());

        self.top_reviews
            .sort_by_key(|r| std::cmp::Reverse(r.votes_helpful));
        self.top_reviews.truncate(10);
    }

    ///Devuelve los top 'n' idiomas más populares en los que se escribieron más reseñasn, dada una cantidad solicitada,
    /// junto con las top 10 reseñas de cada idioma.
    pub fn top_n_languages(
        languages: HashMap<String, Language>,
        num_languages_to_display: usize,
    ) -> Vec<Value> {
        let mut top_languages: Vec<_> = languages.into_iter().collect();
        let mut top_languages_output: Vec<Value> = Vec::new();

        top_languages.sort_by(|a, b| {
            b.1.review_count
                .cmp(&a.1.review_count)
                .then_with(|| a.0.cmp(&b.0))
        });
        top_languages.truncate(num_languages_to_display);

        for (language, lang_data) in &mut top_languages {
            let mut top_reviews_json = Vec::new();
            lang_data.top_reviews.truncate(10);

            for review in lang_data.top_reviews.iter().take(10) {
                let review_entry = json!({
                    "review": review.text.clone(),
                    "votes": review.votes_helpful,
                });
                top_reviews_json.push(review_entry);
            }
            let language_entry = json!({
                "language": language,
                "review_count": lang_data.review_count,
                "top_reviews": top_reviews_json,
            });

            top_languages_output.push(language_entry);
        }

        top_languages_output
    }
}
