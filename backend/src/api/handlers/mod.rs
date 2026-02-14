pub mod cards;
pub mod comments;
pub mod labels;
pub mod sse;
pub mod subtasks;

use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn liveness() -> StatusCode {
    StatusCode::OK
}
