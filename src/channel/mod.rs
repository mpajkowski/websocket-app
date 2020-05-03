use crate::state::State;
use anyhow::Result;
use serde_json::Value;
use std::{fmt::Debug, hash::Hash};

mod reward;
mod thirteen_chan;

pub use reward::Reward;
pub use thirteen_chan::ThirteenChan;

#[async_trait::async_trait]
pub trait Channel: Send + Sync + Debug {
    fn name(&self) -> &str;
    async fn extract_data(&self, state: &State) -> Result<Value>;
}

impl Hash for dyn Channel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name().hash(state)
    }
}

impl PartialEq for dyn Channel {
    fn eq(&self, other: &dyn Channel) -> bool {
        self.name() == other.name()
    }
}

impl Eq for dyn Channel {}
