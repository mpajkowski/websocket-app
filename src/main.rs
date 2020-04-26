#![deny(unused_imports, unused_must_use)]

use std::env;

use anyhow::Result;
use tokio::net::TcpListener;

pub mod accept;
pub mod broker;
pub mod client;
pub mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv::dotenv().ok();

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let listener = TcpListener::bind(&addr).await?;
    log::info!("Listening on: {}", addr);

    crate::accept::accept_loop(listener).await?;

    Ok(())
}
