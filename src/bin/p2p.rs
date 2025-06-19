use log::info;
use tokio::net::TcpListener;
use std::error::Error;

pub async fn init() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:9000").await?;
    info!("P2P network listening on port 9000");
    
    Ok(())
}
