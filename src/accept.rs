use crate::broker::broker_loop;
use crate::client;
use crate::utils::spawn_and_log_err;
use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::mpsc::unbounded_channel;

pub async fn accept_loop(mut listener: TcpListener) -> Result<()> {
    log::debug!("Enter accept_loop");
    let (broker_tx, broker_rx) = unbounded_channel();

    spawn_and_log_err(broker_loop(broker_rx));

    while let Ok((stream, _)) = listener.accept().await {
        spawn_and_log_err(client::handle_connection(stream, broker_tx.clone()));
    }

    Ok(())
}
