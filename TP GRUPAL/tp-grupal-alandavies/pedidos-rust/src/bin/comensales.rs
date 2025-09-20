use actix::prelude::*;
use pedidos_rust::{
    app_config::Config,
    args_parser::args::Args,
    comensales::{comensal::Comensal, nodo_comensal::NodoComensal, udp_oyente::UdpOyente},
    error::Error,
    logger::log::{AppType, Logger},
    mensajes::{addr_nodo::AddrNodo, ver_restaurantes_interno::VerRestaurantesInterno},
};
use std::{collections::HashMap, env, net::SocketAddrV4, path::Path};
use tokio::sync::oneshot;

#[actix::main]
async fn main() -> Result<(), Error> {
    let args = Args::new(env::args().collect())?;
    let config = Config::cargar_config(args.config_path())?;

    let ip_nodo_comensal = SocketAddrV4::new(config.ip_comensales(), args.puerto());
    let gateway_addr = SocketAddrV4::new(config.ip_pagos(), config.puerto_pagos());

    let logger = Logger::new(
        Path::new(config.logger_dir()),
        AppType::Comensal,
        args.puerto(),
        true,
    )?;

    // Restaurantes disponibles desde config
    let mut restaurantes = HashMap::new();
    let (puerto_inicio, puerto_fin) = config.rango_puertos_restaurantes();
    for (id, puerto) in (puerto_inicio..=puerto_fin).enumerate() {
        restaurantes.insert(
            id as u32 + 1,
            SocketAddrV4::new(config.ip_restaurantes(), puerto),
        );
    }

    let ubicacion_comensal = args.ubicacion();

    let (tx, rx) = oneshot::channel();

    let _comensal_addr = Comensal::create(move |ctx| {
        let actor = Comensal::new(1, *ubicacion_comensal, None, logger);
        let _ = tx.send(ctx.address());
        actor
    });

    let comensal_addr = rx.await.unwrap();

    let logger = Logger::new(
        Path::new(config.logger_dir()),
        AppType::Comensal,
        args.puerto(),
        true,
    )?;

    let nodo_comensal_addr = NodoComensal::new(
        ip_nodo_comensal,
        restaurantes,
        gateway_addr,
        comensal_addr.clone(),
        logger,
    )
    .start();

    comensal_addr.do_send(AddrNodo::new(nodo_comensal_addr.clone()));

    let udp_oyente = UdpOyente::new(nodo_comensal_addr.clone(), ip_nodo_comensal);

    // Inicia el flujo de mensajes
    nodo_comensal_addr.do_send(VerRestaurantesInterno::new(1, *ubicacion_comensal));

    let _ = udp_oyente.start().await;

    Ok(())
}
