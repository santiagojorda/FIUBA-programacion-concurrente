use std::path::Path;

use crate::args_parser::error::{ArgsError, ARGUMENTOS_INVALIDOS_PAGOS, PATH_INVALIDO};

pub struct Args {
    config_path: String,
}

impl Args {
    pub fn new(args: Vec<String>) -> Result<Args, ArgsError> {
        if args.len() != 2 {
            return Err(ArgsError::InvalidArgs(
                ARGUMENTOS_INVALIDOS_PAGOS.to_string(),
            ));
        }
        let config_path = Path::new(&args[1]);
        if !config_path.exists() {
            return Err(ArgsError::InvalidConfigPath(
                PATH_INVALIDO.to_string() + &args[1],
            ));
        }
        Ok(Args {
            config_path: args[1].to_string(),
        })
    }

    pub fn config_path(&self) -> &str {
        &self.config_path
    }
}
