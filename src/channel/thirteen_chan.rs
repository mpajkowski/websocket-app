use super::Channel;
use crate::state::State;
use anyhow::Result;
use serde_json::{json, Value};
use sqlx::prelude::*;

#[derive(Debug)]
pub struct ThirteenChan {}

impl ThirteenChan {
    async fn get(&self, state: &State) -> Result<Value> {
        let row: (Option<String>,) =
            sqlx::query_as("SELECT payload FROM state WHERE channel = '13'")
                .fetch_one(&state.pool)
                .await?;

        let result = match row.0 {
            Some(res) => serde_json::from_str(&res)?,
            None => json!({}),
        };

        Ok(result)
    }
}

#[async_trait::async_trait]
impl Channel for ThirteenChan {
    fn name(&self) -> &str {
        "13"
    }

    async fn extract_data(&self, state: &State) -> Result<serde_json::Value> {
        let res = self.get(&state).await?;
        Ok(res)
    }
}
