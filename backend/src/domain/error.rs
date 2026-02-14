use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum KanbanError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl IntoResponse for KanbanError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            KanbanError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            KanbanError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            KanbanError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            KanbanError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".into(),
                )
            }
            KanbanError::Serialization(err) => {
                tracing::error!("Serialization error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".into(),
                )
            }
        };

        let body = json!({
            "error": message,
            "status": status.as_u16(),
        });

        (status, axum::Json(body)).into_response()
    }
}
