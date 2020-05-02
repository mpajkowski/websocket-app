use crate::state::State;
use serde_json::Value;
use std::{fmt::Debug, hash::Hash};

mod reward;

pub use reward::Reward;

pub trait Channel: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn extract_data(&self, state: &State) -> Value;
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
