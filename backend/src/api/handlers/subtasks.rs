use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::dto::{CreateSubtaskRequest, UpdateSubtaskRequest};
use crate::api::AppState;
use crate::domain::{KanbanError, Subtask};
use crate::services::CardService;

pub async fn create_subtask(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
    Json(req): Json<CreateSubtaskRequest>,
) -> Result<(StatusCode, Json<Subtask>), KanbanError> {
    let pool = state.require_db()?;
    let subtask = CardService::create_subtask(pool, &card_id, req).await?;
    Ok((StatusCode::CREATED, Json(subtask)))
}

pub async fn update_subtask(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSubtaskRequest>,
) -> Result<Json<Subtask>, KanbanError> {
    let pool = state.require_db()?;
    let subtask = CardService::update_subtask(pool, &id, req).await?;
    Ok(Json(subtask))
}

pub async fn delete_subtask(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    CardService::delete_subtask(pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
