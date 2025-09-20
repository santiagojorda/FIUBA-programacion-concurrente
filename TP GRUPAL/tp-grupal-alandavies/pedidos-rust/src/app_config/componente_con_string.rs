use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ComponenteConString {
    directorio: String,
}

impl ComponenteConString {
    pub fn directorio(&self) -> &str {
        &self.directorio
    }
}
