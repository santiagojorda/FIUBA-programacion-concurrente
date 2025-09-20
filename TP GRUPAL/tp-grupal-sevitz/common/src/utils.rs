use std::net::SocketAddr;

pub fn socket_addr_from_string(addr: String) -> SocketAddr {
    match addr.parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Error parsing address: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn get_rand_f32_tuple() -> (f32, f32) {
    (
        (rand::random::<f32>() * 20.0).round(),
        (rand::random::<f32>() * 20.0).round(),
    )
}
