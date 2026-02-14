use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::SqlitePool;

use crate::api::dto::{BoardResponse, CardResponse, CreateCardRequest, MoveCardRequest, UpdateCardRequest};
use crate::api::handlers::sse::SseEvent;
use crate::api::AppState;
use crate::domain::{Card, Comment, KanbanError, Stage};
use crate::services::{AiDispatchService, CardService};

pub async fn create_card(
    State(state): State<AppState>,
    Json(req): Json<CreateCardRequest>,
) -> Result<(StatusCode, Json<CardResponse>), KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::create_card(pool, req).await?;
    Ok((StatusCode::CREATED, Json(card)))
}

pub async fn get_board(
    State(state): State<AppState>,
) -> Result<Json<BoardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let board = CardService::get_board(pool).await?;
    Ok(Json(board))
}

pub async fn get_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_by_id(pool, &id).await?;
    Ok(Json(card))
}

pub async fn update_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCardRequest>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::update_card(pool, &id, req).await?;
    Ok(Json(card))
}

pub async fn move_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<MoveCardRequest>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let previous_card = CardService::get_card_model(pool, &id).await?;

    let target_stage: Stage = req.stage.parse()
        .map_err(|e: String| KanbanError::BadRequest(e))?;
    let current_stage: Stage = previous_card.stage.parse()
        .map_err(|e: String| KanbanError::Internal(format!("Invalid current stage in DB: {}", e)))?;

    if !current_stage.can_transition_to(&target_stage) {
        return Err(KanbanError::BadRequest(current_stage.transition_error(&target_stage)));
    }

    let is_review_to_todo = current_stage == Stage::Review && target_stage == Stage::Todo;

    let card = CardService::move_card(pool, &id, req).await?;

    if target_stage == Stage::Todo && previous_card.stage != "todo" {
        if is_review_to_todo {
            if let Err(e) = handle_review_redispatch(&state, &previous_card, pool).await {
                tracing::warn!("Review re-dispatch failed for card {}: {}", id, e);
            }
        } else {
            let card_model = CardService::get_card_model(pool, &id).await?;
            let subtasks = CardService::get_subtasks(pool, &id).await?;

            if let Err(e) =
                AiDispatchService::new(state.http_client.clone(), state.config.opencode_url.clone())
                    .dispatch_card(&card_model, &subtasks, pool)
                    .await
            {
                tracing::warn!("AI dispatch failed for card {}: {}", id, e);
            }
        }

        let updated_card = CardService::get_card_by_id(pool, &id).await?;

        let event = SseEvent::AiStatusChanged {
            card_id: id,
            status: updated_card.ai_status.clone(),
            progress: updated_card.ai_progress.clone(),
        };
        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = state.sse_tx.send(payload);
        }

        return Ok(Json(updated_card));
    }

    Ok(Json(card))
}

async fn handle_review_redispatch(
    state: &AppState,
    card: &Card,
    pool: &SqlitePool,
) -> Result<(), KanbanError> {
    let comments: Vec<Comment> = sqlx::query_as(
        "SELECT * FROM comments WHERE card_id = ? ORDER BY created_at DESC LIMIT 5",
    )
    .bind(&card.id)
    .fetch_all(pool)
    .await?;

    let plan_path = card
        .plan_path
        .as_ref()
        .ok_or_else(|| KanbanError::Internal("No plan path for re-dispatch".into()))?;

    let existing_plan = std::fs::read_to_string(plan_path)
        .map_err(|e| KanbanError::Internal(format!("Failed to read plan: {}", e)))?;

    let mut updated_plan = existing_plan;
    updated_plan.push_str("\n\n---\n\n## Review Feedback\n\n");
    updated_plan.push_str("The following feedback was provided during review:\n\n");
    for comment in &comments {
        updated_plan.push_str(&format!("- **{}**: {}\n", comment.author, comment.content));
    }
    updated_plan.push_str(
        "\n**Action Required**: Address the review feedback above and re-verify all acceptance criteria.\n",
    );

    std::fs::write(plan_path, &updated_plan)
        .map_err(|e| KanbanError::Internal(format!("Failed to write plan: {}", e)))?;

    let subtasks = CardService::get_subtasks(pool, &card.id).await?;

    AiDispatchService::new(state.http_client.clone(), state.config.opencode_url.clone())
        .dispatch_card(card, &subtasks, pool)
        .await?;

    Ok(())
}

pub async fn delete_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    CardService::delete_card(pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
