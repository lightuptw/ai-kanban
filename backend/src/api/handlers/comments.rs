use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::dto::CreateCommentRequest;
use crate::api::AppState;
use crate::domain::{Comment, KanbanError};
use crate::services::CardService;

pub async fn create_comment(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<Comment>), KanbanError> {
    let pool = state.require_db()?;
    let comment = CardService::create_comment(pool, &card_id, req).await?;
    Ok((StatusCode::CREATED, Json(comment)))
}
