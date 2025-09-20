//! Módulo para manejar los datos estadísticos las reseñas mejores puntuadas por los usuarios.
use serde::Serialize;

#[derive(Clone, Serialize)]
//Estructura para almacenar los datos de uan reseña escrita por un usuario acerca de un juego.
/// ###  text
/// Se almacena el texto escrito por el usuario.
/// ###  votes_helpful
/// Se almacenan la cantidad de votos (positivos) que recibió la reseña por parte de otros usuarios.
pub struct TopReview {
    pub text: String,
    pub votes_helpful: u32,
}

impl TopReview {
    ///Función para crea runa nueva instancia de TopReview
    pub fn new(user_review: String, votes_helpful: u32) -> TopReview {
        TopReview {
            text: user_review,
            votes_helpful,
        }
    }
}
