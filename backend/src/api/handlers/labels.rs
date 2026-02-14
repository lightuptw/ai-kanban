use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::AppState;
use crate::domain::{KanbanError, Label};
use crate::services::CardService;

pub async fn list_labels(
    State(state): State<AppState>,
) -> Result<Json<Vec<Label>>, KanbanError> {
    let pool = state.require_db()?;
    let labels = CardService::list_labels(pool).await?;
    Ok(Json(labels))
}

pub async fn add_label(
    State(state): State<AppState>,
    Path((card_id, label_id)): Path<(String, String)>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    CardService::add_label_to_card(pool, &card_id, &label_id).await?;
    Ok(StatusCode::CREATED)
}

pub async fn remove_label(
    State(state): State<AppState>,
    Path((card_id, label_id)): Path<(String, String)>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    CardService::remove_label_from_card(pool, &card_id, &label_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
