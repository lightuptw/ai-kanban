use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum KanbanError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("OpenCode error: {0}")]
    OpenCodeError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for KanbanError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            KanbanError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            KanbanError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            KanbanError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            KanbanError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            KanbanError::OpenCodeError(msg) => (StatusCode::BAD_GATEWAY, msg),
            KanbanError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
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
