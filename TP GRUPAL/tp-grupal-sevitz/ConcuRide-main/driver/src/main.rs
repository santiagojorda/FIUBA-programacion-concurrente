mod driver;

use driver::Driver;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut servers: Vec<SocketAddr> = Vec::new();

    for port in 8080..=8084 {
        let server: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
        servers.push(server);
    }

    let mut driver = Driver::new(servers).await;
    driver.run().await;

    Ok(())
}
