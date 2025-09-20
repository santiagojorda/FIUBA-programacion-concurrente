use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Ubicacion(u32, u32);

impl fmt::Display for Ubicacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ubicacion({}, {})", self.0, self.1)
    }
}

impl Ubicacion {
    pub fn new(latitud: u32, longitud: u32) -> Self {
        Ubicacion(latitud, longitud)
    }

    pub fn latitud(&self) -> u32 {
        self.0
    }

    pub fn longitud(&self) -> u32 {
        self.1
    }

    pub fn distancia(&self, otra: &Ubicacion) -> f32 {
        let dx = (self.latitud() as f32 - otra.latitud() as f32).powi(2);
        let dy = (self.longitud() as f32 - otra.longitud() as f32).powi(2);
        (dx + dy).sqrt()
    }
}
