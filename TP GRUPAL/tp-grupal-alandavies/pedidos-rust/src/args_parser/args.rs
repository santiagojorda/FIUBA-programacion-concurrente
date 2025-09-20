use std::{num::ParseIntError, path::Path};

use crate::{
    args_parser::error::{ArgsError, ARGUMENTOS_INVALIDOS, PATH_INVALIDO},
    estructuras_aux::ubicacion::Ubicacion,
};

pub struct Args {
    config_path: String,
    puerto: u16,
    ubicacion: Ubicacion,
}

impl Args {
    pub fn new(args: Vec<String>) -> Result<Args, ArgsError> {
        if args.len() != 5 {
            return Err(ArgsError::InvalidArgs(ARGUMENTOS_INVALIDOS.to_string()));
        }
        let config_path = Path::new(&args[1]);
        if !config_path.exists() {
            return Err(ArgsError::InvalidConfigPath(
                PATH_INVALIDO.to_string() + &args[1],
            ));
        }
        let puerto: u16 = args[2]
            .parse()
            .map_err(|e: ParseIntError| ArgsError::InvalidPort(e.to_string()))?;
        let x: u32 = args[3].parse().map_err(ArgsError::InvalidCoord)?;
        let y: u32 = args[4].parse().map_err(ArgsError::InvalidCoord)?;
        Ok(Args {
            config_path: args[1].to_string(),
            puerto,
            ubicacion: Ubicacion::new(x, y),
        })
    }

    pub fn config_path(&self) -> &str {
        &self.config_path
    }

    pub fn puerto(&self) -> u16 {
        self.puerto
    }

    pub fn ubicacion(&self) -> &Ubicacion {
        &self.ubicacion
    }
}
