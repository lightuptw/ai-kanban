use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::handlers::sse::WsEvent;
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

    let event = WsEvent::LabelAdded {
        card_id: card_id.clone(),
        label_id: label_id.clone(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok(StatusCode::CREATED)
}

pub async fn remove_label(
    State(state): State<AppState>,
    Path((card_id, label_id)): Path<(String, String)>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    CardService::remove_label_from_card(pool, &card_id, &label_id).await?;

    let event = WsEvent::LabelRemoved {
        card_id: card_id.clone(),
        label_id: label_id.clone(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok(StatusCode::NO_CONTENT)
}
