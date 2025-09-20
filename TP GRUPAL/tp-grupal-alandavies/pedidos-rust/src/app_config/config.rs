use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::net::Ipv4Addr;

use crate::app_config::componente::Componente;
use crate::app_config::componente_con_puerto::ComponenteConPuerto;
use crate::app_config::componente_con_rango::ComponenteConRango;
use crate::app_config::componente_con_string::ComponenteConString;
use crate::app_config::error::{
    ConfigError, ERROR_IPS_DUPLICADAS, ERROR_RANGO_PUERTOS_REPARTIDORES,
    ERROR_RANGO_PUERTOS_RESTAURANTES,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    comensales: Componente,
    repartidores: ComponenteConRango,
    restaurantes: ComponenteConRango,
    pagos: ComponenteConPuerto,
    logger: ComponenteConString,
}

impl Config {
    fn validar(&self) -> Result<(), ConfigError> {
        if !self.repartidores.validar_rango_puertos() {
            return Err(ConfigError::validation_error(
                ERROR_RANGO_PUERTOS_REPARTIDORES,
            ));
        }
        if !self.restaurantes.validar_rango_puertos() {
            return Err(ConfigError::validation_error(
                ERROR_RANGO_PUERTOS_RESTAURANTES,
            ));
        }
        let mut ips: HashSet<Ipv4Addr> = HashSet::new();
        ips.insert(self.ip_restaurantes());
        ips.insert(self.ip_repartidores());
        ips.insert(self.ip_pagos());
        ips.insert(self.ip_comensales());
        if ips.len() != 4 {
            return Err(ConfigError::validation_error(ERROR_IPS_DUPLICADAS));
        }
        Ok(())
    }

    pub fn cargar_config(path: &str) -> Result<Config, ConfigError> {
        let contenido = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contenido)?;
        config.validar()?;
        Ok(config)
    }

    pub fn ip_comensales(&self) -> Ipv4Addr {
        self.comensales.ip()
    }

    pub fn ip_restaurantes(&self) -> Ipv4Addr {
        self.restaurantes.ip()
    }

    pub fn ip_repartidores(&self) -> Ipv4Addr {
        self.repartidores.ip()
    }

    pub fn ip_pagos(&self) -> Ipv4Addr {
        self.pagos.ip()
    }

    pub fn rango_puertos_restaurantes(&self) -> (u16, u16) {
        self.restaurantes.rango_puertos()
    }

    pub fn rango_puertos_repartidores(&self) -> (u16, u16) {
        self.repartidores.rango_puertos()
    }

    pub fn puerto_pagos(&self) -> u16 {
        self.pagos.puerto()
    }

    pub fn logger_dir(&self) -> &str {
        self.logger.directorio()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;

    fn crear_archivo_config(id: u8) -> (PathBuf, PathBuf) {
        let log_dir = env::temp_dir().join(format!("test_logs_{}", id));
        fs::create_dir_all(&log_dir).expect("Failed to create test directory");

        let contenido = format!(
            r#"
        [comensales]
        ip = "192.168.1.10"

        [repartidores]
        ip = "192.168.1.20"
        rango_puertos = [1000, 1050]

        [restaurantes]
        ip = "192.168.1.30"
        rango_puertos = [2000, 2050]

        [pagos]
        ip = "192.168.1.40"
        puerto = 8080

        [logger]
        directorio = "{}"
        "#,
            log_dir.to_str().unwrap()
        );

        let file_path = env::temp_dir().join("test_config.toml");
        fs::write(&file_path, contenido).unwrap();

        (file_path, log_dir)
    }

    #[test]
    fn test_cargar_config_valida() {
        let (file_path, log_dir) = crear_archivo_config(0);
        let config = Config::cargar_config(file_path.to_str().unwrap()).unwrap();
        println!("{:?}", config.logger.directorio());

        assert_eq!(
            config.comensales.ip(),
            "192.168.1.10".parse::<Ipv4Addr>().unwrap()
        );
        assert_eq!(
            config.repartidores.ip(),
            "192.168.1.20".parse::<Ipv4Addr>().unwrap()
        );
        assert_eq!(config.repartidores.rango_puertos(), (1000, 1050));
        assert_eq!(
            config.restaurantes.ip(),
            "192.168.1.30".parse::<Ipv4Addr>().unwrap()
        );
        assert_eq!(config.restaurantes.rango_puertos(), (2000, 2050));
        assert_eq!(
            config.pagos.ip(),
            "192.168.1.40".parse::<Ipv4Addr>().unwrap()
        );
        assert_eq!(config.pagos.puerto(), 8080);
        assert_eq!(config.logger.directorio(), log_dir.to_str().unwrap());

        fs::remove_file(file_path).unwrap();
        fs::remove_dir_all(log_dir).expect("Failed to remove test directory");
    }

    #[test]
    fn test_cargar_config_archivo_no_existe() {
        let resultado = Config::cargar_config("archivo_inexistente.toml");
        assert!(matches!(resultado, Err(ConfigError::IoError(_))));
    }

    #[test]
    fn test_cargar_config_formato_invalido() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("invalid_config.toml");
        fs::write(&file_path, "formato invalido").unwrap();

        let resultado = Config::cargar_config(file_path.to_str().unwrap());
        assert!(matches!(resultado, Err(ConfigError::ParseError(_))));

        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_cargar_config_ip_invalida() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("invalid_ip_config.toml");
        let contenido = r#"
            [comensales]
            ip = "256.168.1.10"

            [repartidores]
            ip = "192.168.1.20"
            rango_puertos = [1000, 1050]

            [restaurantes]
            ip = "192.168.1.30"
            rango_puertos = [2000, 2050]

            [pagos]
            ip = "192.168.1.40"
            puerto = 8080

            [logger]
            directorio = "/var/log/app"
        "#;
        fs::write(&file_path, contenido).unwrap();

        let resultado = Config::cargar_config(file_path.to_str().unwrap());
        assert!(matches!(resultado, Err(ConfigError::ParseError(_))));

        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_cargar_config_rango_puertos_invalido() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("invalid_ports_config.toml");
        let contenido = r#"
            [comensales]
            ip = "192.168.1.10"

            [repartidores]
            ip = "192.168.1.20"
            rango_puertos = [1050, 1000]

            [restaurantes]
            ip = "192.168.1.30"
            rango_puertos = [2000, 2050]

            [pagos]
            ip = "192.168.1.40"
            puerto = 8080

            [logger]
            directorio = "/var/log/app"
        "#;
        fs::write(&file_path, contenido).unwrap();

        let resultado = Config::cargar_config(file_path.to_str().unwrap());
        assert!(matches!(resultado, Err(ConfigError::ValidationError(_))));

        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_cargar_config_ip_duplicada() {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("duplicated_ip_config.toml");
        let contenido = r#"
            [comensales]
            ip = "192.168.1.10"

            [repartidores]
            ip = "192.168.1.20"
            rango_puertos = [1000, 1050]

            [restaurantes]
            ip = "192.168.1.30"
            rango_puertos = [2000, 2050]

            [pagos]
            ip = "192.168.1.30"
            puerto = 8080

            [logger]
            directorio = "/var/log/app"
        "#;
        fs::write(&file_path, contenido).unwrap();

        let resultado = Config::cargar_config(file_path.to_str().unwrap());
        assert!(matches!(resultado, Err(ConfigError::ValidationError(_))));

        fs::remove_file(file_path).unwrap();
    }
}
