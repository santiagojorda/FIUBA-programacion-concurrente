use actix::prelude::*;
use pedidos_rust::{
    app_config::Config,
    error::Error,
    gateway_pagos::{args::Args, pagos::GatewayPagos, tcp_oyente::TcpOyente},
    logger::log::{AppType, Logger},
};
use std::{env, fs::OpenOptions, net::SocketAddrV4, path::Path};

#[actix::main]
async fn main() -> Result<(), Error> {
    let args = Args::new(env::args().collect())?;
    let config = Config::cargar_config(args.config_path())?;
    let local = SocketAddrV4::new(config.ip_pagos(), config.puerto_pagos());
    let logger = Logger::new(Path::new(config.logger_dir()), AppType::Pagos, 8080, true)?;

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("pagos.log")
        .expect("No se pudo abrir el archivo de log");

    let gateway_addr = GatewayPagos::new(log_file, logger).start();

    let oyente = TcpOyente::new(gateway_addr, local);

    oyente.start().await.map_err(Error::from)?;
    Ok(())
}
