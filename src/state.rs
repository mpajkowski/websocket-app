use serde_json::json;
use serde_json::Value;
use sqlx::SqlitePool;

/// Dummy state
pub struct State {
    pub pool: SqlitePool,
    pub static_data: Value,
}

impl State {
    pub fn new(pool: SqlitePool) -> State {
        State {
            pool,
            static_data: json!({"version": "alpha"}),
        }
    }
}
