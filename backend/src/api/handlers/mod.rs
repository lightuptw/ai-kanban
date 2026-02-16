pub mod board_settings;
pub mod boards;
pub mod cards;
pub mod comments;
pub mod files;
pub mod labels;
pub mod notifications;
pub mod picker;
pub mod questions;
pub mod settings;
pub mod sse;
pub mod subtasks;
pub mod ws;

use axum::Json;
use serde_json::{json, Value};

pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
