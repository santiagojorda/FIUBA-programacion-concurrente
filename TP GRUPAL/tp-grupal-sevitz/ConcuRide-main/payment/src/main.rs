mod payment_gateway;
use payment_gateway::PaymentGatewayActor;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:8085".to_string().parse();
    let addr = match addr {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to parse address: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, e));
        }
    };

    PaymentGatewayActor::start(addr).await?;

    Ok(())
}
