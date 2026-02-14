use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Deserialize;

use crate::api::dto::CreateCommentRequest;
use crate::api::AppState;
use crate::domain::{Comment, KanbanError};
use crate::services::CardService;

#[derive(Debug, Deserialize)]
pub struct UpdateCommentRequest {
    pub content: String,
}

pub async fn get_comments(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
) -> Result<Json<Vec<Comment>>, KanbanError> {
    let pool = state.require_db()?;
    let comments = sqlx::query_as::<_, Comment>(
        "SELECT * FROM comments WHERE card_id = ? ORDER BY created_at ASC"
    )
    .bind(&card_id)
    .fetch_all(pool)
    .await?;
    Ok(Json(comments))
}

pub async fn create_comment(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<Comment>), KanbanError> {
    let pool = state.require_db()?;
    let comment = CardService::create_comment(pool, &card_id, req).await?;
    Ok((StatusCode::CREATED, Json(comment)))
}

pub async fn update_comment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCommentRequest>,
) -> Result<Json<Comment>, KanbanError> {
    let pool = state.require_db()?;
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query("UPDATE comments SET content = ?, created_at = ? WHERE id = ?")
        .bind(&req.content)
        .bind(&now)
        .bind(&id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(KanbanError::NotFound(format!("Comment not found: {}", id)));
    }
    let comment = sqlx::query_as::<_, Comment>("SELECT * FROM comments WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?;
    Ok(Json(comment))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    let result = sqlx::query("DELETE FROM comments WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(KanbanError::NotFound(format!("Comment not found: {}", id)));
    }
    Ok(StatusCode::NO_CONTENT)
}
