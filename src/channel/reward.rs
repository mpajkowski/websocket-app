use super::Channel;
use crate::state::State;

#[derive(Debug)]
pub struct Reward {}

impl Channel for Reward {
    fn name(&self) -> &str {
        "reward"
    }

    fn extract_data(&self, state: &State) -> serde_json::Value {
        state.data.clone()
    }
}
