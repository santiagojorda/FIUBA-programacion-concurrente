use actix::prelude::*;
use pedidos_rust::app_config::Config;
use pedidos_rust::args_parser::args::Args;
use pedidos_rust::args_parser::error::{ArgsError, PUERTO_RANGO_INVALIDO};
use pedidos_rust::error::Error;
use pedidos_rust::logger::log::{AppType, Logger};
use pedidos_rust::mensajes::addr_nodo::AddrNodo;
use pedidos_rust::repartidores::nodo_repartidor::NodoRepartidor;
use pedidos_rust::repartidores::repartidor::Repartidor;
use pedidos_rust::repartidores::tcp_oyente::TcpOyente;
use rand::{thread_rng, Rng};
use std::env;
use std::net::SocketAddrV4;
use std::path::Path;
use std::process::{Child, Command};
use tokio::sync::oneshot;
use tokio::task::LocalSet;

async fn lanzar_repartidor_especifico(raw_args: Vec<String>) -> Result<(), Error> {
    let args = Args::new(raw_args)?;
    let ubicacion = args.ubicacion();
    let config = Config::cargar_config(args.config_path())?;
    let pagos = SocketAddrV4::new(config.ip_pagos(), config.puerto_pagos());
    let local = SocketAddrV4::new(config.ip_repartidores(), args.puerto());
    if args.puerto() < config.rango_puertos_repartidores().0
        || args.puerto() > config.rango_puertos_repartidores().1
    {
        return Err(ArgsError::InvalidPort(PUERTO_RANGO_INVALIDO.to_string()).into());
    }
    let logger_nodo = Logger::new(
        Path::new(config.logger_dir()),
        AppType::NodoRepartidor,
        args.puerto(),
        true,
    )?;
    let logger_repartidor = Logger::new(
        Path::new(config.logger_dir()),
        AppType::Repartidor,
        args.puerto(),
        true,
    )?;
    let local_set = LocalSet::new();
    local_set
        .run_until(async {
            let (tx, rx) = oneshot::channel();
            let repartidor = Repartidor::create(move |ctx| {
                let actor = Repartidor::new(ubicacion, logger_repartidor);
                let _ = tx.send(ctx.address());
                actor
            });
            if let Ok(addr_repartidor) = rx.await {
                let nodo_addr = NodoRepartidor::new(
                    repartidor,
                    pagos,
                    local,
                    config.rango_puertos_repartidores(),
                    config.ip_restaurantes(),
                    config.rango_puertos_restaurantes(),
                    logger_nodo,
                )
                .start();
                addr_repartidor.do_send(AddrNodo::new(nodo_addr.clone()));
                let oyente_tcp = TcpOyente::new(nodo_addr, local);
                tokio::task::spawn_local(async move {
                    let _ = oyente_tcp.start().await;
                });
                tokio::signal::ctrl_c().await?;
            }
            Ok::<(), Error>(())
        })
        .await?;
    Ok(())
}

fn lanzar_repartidores_en_rango(raw_args: Vec<String>) -> Result<(), Error> {
    const MIN_LOCATION_COORD: u32 = 1;
    const MAX_LOCATION_COORD: u32 = 120;

    let config_for_spawner = Config::cargar_config(&raw_args[1])?;

    let mut children: Vec<Child> = Vec::new();
    let mut rng = thread_rng();

    let (min_port, mut max_port) = config_for_spawner.rango_puertos_repartidores();

    if raw_args.len() == 3 {
        if let Ok(cantidad) = &raw_args[2].parse() {
            max_port = min_port + cantidad - 1
        }
    }

    println!("Iniciando el spawner de repartidores...");
    println!(
        "Lanzando repartidores desde el puerto {} hasta el {} con ubicaciones aleatorias ({}-{})",
        min_port, max_port, MIN_LOCATION_COORD, MAX_LOCATION_COORD
    );

    // Itera sobre el rango de puertos para lanzar una instancia de restaurante por cada uno.
    for port in min_port..=max_port {
        let rand_x = rng.gen_range(MIN_LOCATION_COORD..=MAX_LOCATION_COORD);
        let rand_y = rng.gen_range(MIN_LOCATION_COORD..=MAX_LOCATION_COORD);

        println!(
            "Lanzando: cargo run --bin repartidores -- {} {} {} {}",
            &raw_args[1], port, rand_x, rand_y
        );
        let child = Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("repartidores")
            .arg("--")
            .arg(&raw_args[1]) // Pasa la ruta de configuración al proceso hijo
            .arg(port.to_string()) // Pasa el puerto actual al proceso hijo
            .arg(rand_x.to_string()) // Pasa la coordenada X aleatoria al proceso hijo
            .arg(rand_y.to_string()) // Pasa la coordenada Y aleatoria al proceso hijo
            .spawn()
            .expect("Fallo al iniciar el proceso hijo del repartidor");
        children.push(child);
    }

    for mut child in children {
        let _ = child.wait()?;
    }

    println!("Todos los procesos de restaurantes lanzados han finalizado.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let raw_args = env::args().collect::<Vec<String>>();
    if raw_args.len() == 2 || raw_args.len() == 3 {
        lanzar_repartidores_en_rango(raw_args)?;
    } else {
        lanzar_repartidor_especifico(raw_args).await?;
    }
    Ok(())
}
