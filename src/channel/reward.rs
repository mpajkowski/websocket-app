use super::Channel;
use crate::state::State;
use anyhow::Result;

#[derive(Debug)]
pub struct Reward {}

#[async_trait::async_trait]
impl Channel for Reward {
    fn name(&self) -> &str {
        "reward"
    }

    async fn extract_data(&self, state: &State) -> Result<serde_json::Value> {
        let res = state.static_data.clone();
        Ok(res)
    }
}
