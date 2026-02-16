use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::api::AppState;
use crate::domain::{KanbanError, Notification};
use crate::services::NotificationService;

#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    #[serde(default)]
    pub unread_only: Option<bool>,
}

pub async fn list_notifications(
    State(state): State<AppState>,
    Query(query): Query<ListNotificationsQuery>,
) -> Result<Json<Vec<Notification>>, KanbanError> {
    let pool = state.require_db()?;
    let notifications =
        NotificationService::list_notifications(pool, None, query.unread_only.unwrap_or(false))
            .await?;
    Ok(Json(notifications))
}

pub async fn mark_read(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Notification>, KanbanError> {
    let pool = state.require_db()?;
    let notification = NotificationService::mark_read(pool, &id).await?;
    Ok(Json(notification))
}

pub async fn mark_all_read(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, KanbanError> {
    let pool = state.require_db()?;
    let count = NotificationService::mark_all_read(pool, None).await?;
    Ok(Json(serde_json::json!({ "marked_read": count })))
}

pub async fn delete_notification(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, KanbanError> {
    let pool = state.require_db()?;
    NotificationService::delete_notification(pool, &id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
