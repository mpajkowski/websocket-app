use crate::channel::Reward;
use crate::client;
use crate::{broker::Broker, state::State, utils::spawn_and_log_err};
use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc::unbounded_channel, RwLock};

pub async fn event_loop(mut listener: TcpListener) -> Result<()> {
    log::debug!("Enter event_loop");

    // initialize dummy state (protected by RwLock)
    let state = Arc::new(RwLock::new(State {
        data: json!({"reward": "$100"}),
    }));

    let (broker_tx, broker_rx) = unbounded_channel();
    let mut broker = Broker::new(broker_rx, state);

    // publish dummy channel
    broker.add_channel(Arc::new(Reward {}));

    // borrow the broker for 'static and spawn its worker future
    spawn_and_log_err(async move { broker.worker().await });

    // asynchronously accept incoming TCP streams
    while let Ok((stream, _)) = listener.accept().await {
        spawn_and_log_err(client::handle_connection(stream, broker_tx.clone()));
    }

    Ok(())
}
