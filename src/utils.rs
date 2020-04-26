use anyhow::Result;
use std::future::Future;
use tokio::task::JoinHandle;

pub fn spawn_and_log_err<F>(fut: F) -> JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + Sync + 'static,
{
    tokio::spawn(async move {
        if let Err(e) = fut.await {
            log::error!("an error occured: {}", e);
        }
    })
}
