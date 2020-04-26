use crate::broker::Event;
use anyhow::{Context, Result};
use futures::{StreamExt, TryStreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;

pub async fn handle_connection(
    raw_stream: TcpStream,
    broker_tx: UnboundedSender<Event>,
) -> Result<()> {
    let addr = raw_stream.peer_addr()?;
    log::info!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .with_context(|| "Error during the websocket handshake occurred")?;

    log::info!("WebSocket connection established: {}", addr);

    let (outgoing, mut incoming) = ws_stream.split();

    // subscribe to broker
    broker_tx.send(Event::NewPeer { addr, tx: outgoing })?;

    while let Some(msg) = incoming.try_next().await? {
        log::info!("Received msg: {:?}", msg);

        broker_tx.send(Event::WireMessage { addr, msg })?;
    }

    log::info!("{} disconnected", &addr);

    Ok(())
}
