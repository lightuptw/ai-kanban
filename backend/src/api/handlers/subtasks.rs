use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx;

use crate::api::dto::{CreateSubtaskRequest, UpdateSubtaskRequest};
use crate::api::handlers::sse::SseEvent;
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

    let event = SseEvent::SubtaskCreated {
        card_id: card_id.clone(),
        subtask: serde_json::to_value(&subtask).unwrap_or_default(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok((StatusCode::CREATED, Json(subtask)))
}

pub async fn update_subtask(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSubtaskRequest>,
) -> Result<Json<Subtask>, KanbanError> {
    let pool = state.require_db()?;
    let card_id = sqlx::query_scalar::<_, String>("SELECT card_id FROM subtasks WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?;

    let subtask = CardService::update_subtask(pool, &id, req).await?;

    let event = SseEvent::SubtaskUpdated {
        card_id,
        subtask: serde_json::to_value(&subtask).unwrap_or_default(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok(Json(subtask))
}

pub async fn delete_subtask(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    let card_id = sqlx::query_scalar::<_, String>("SELECT card_id FROM subtasks WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?;

    CardService::delete_subtask(pool, &id).await?;

    let event = SseEvent::SubtaskDeleted {
        card_id,
        subtask_id: id.clone(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok(StatusCode::NO_CONTENT)
}
