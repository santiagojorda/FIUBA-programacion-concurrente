mod admin_actor;
mod coordinator_actor;
mod elections;
mod storage_actor;
mod utils;
use admin_actor::admin::Admin;
use common::utils::socket_addr_from_string;
use std::net::SocketAddr;
use utils::admin_errors::AdminError;

const PORT_ARG: usize = 1;

#[actix_rt::main]
async fn main() -> Result<(), AdminError> {
    let port = std::env::args().nth(PORT_ARG).unwrap_or("8080".to_string());

    let addr = socket_addr_from_string(format!("127.0.0.1:{}", port));
    let peers: Vec<SocketAddr> = (8080..8085)
        .map(|port| format!("127.0.0.1:{}", port))
        .map(socket_addr_from_string)
        .collect();

    Admin::start(addr, peers).await?;

    Ok(())
}
