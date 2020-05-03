use crate::channel::{Reward, ThirteenChan};
use crate::client;
use crate::{broker::Broker, state::State, utils::spawn_and_log_err};
use anyhow::{anyhow, Result};
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc::unbounded_channel;

pub async fn event_loop(mut listener: TcpListener) -> Result<()> {
    let db_string = env::var("SQLITE_PATH").map_err(|_| anyhow!("Missing path to sqlite db"))?;

    let pool = SqlitePool::builder().max_size(5).build(&db_string).await?;
    let state = State::new(pool);

    let (broker_tx, broker_rx) = unbounded_channel();
    let mut broker = Broker::new(broker_rx, state);

    broker.add_channel(Arc::new(Reward {}));
    broker.add_channel(Arc::new(ThirteenChan {}));

    log::debug!("Enter event_loop");
    // borrow the broker for 'static and spawn its worker future
    spawn_and_log_err(async move { broker.worker().await });

    // asynchronously accept incoming TCP streams
    while let Ok((stream, _)) = listener.accept().await {
        spawn_and_log_err(client::handle_connection(stream, broker_tx.clone()));
    }

    Ok(())
}
