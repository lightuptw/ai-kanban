use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

pub async fn list_cards() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(json!({"error": "Not implemented yet"})),
    )
}

pub async fn create_card() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(json!({"error": "Not implemented yet"})),
    )
}

pub async fn get_card() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(json!({"error": "Not implemented yet"})),
    )
}

pub async fn update_card() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(json!({"error": "Not implemented yet"})),
    )
}

pub async fn delete_card() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(json!({"error": "Not implemented yet"})),
    )
}

pub async fn move_card() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(json!({"error": "Not implemented yet"})),
    )
}
