use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::api::dto::{BoardResponse, CardResponse, CreateCardRequest, MoveCardRequest, UpdateCardRequest};
use crate::api::handlers::sse::SseEvent;
use crate::api::AppState;
use crate::domain::{AgentLog, Card, Comment, KanbanError, Stage};
use crate::services::{AiDispatchService, CardService};

#[derive(Debug, Deserialize)]
pub struct BoardQuery {
    pub board_id: Option<String>,
}

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
    Query(query): Query<BoardQuery>,
) -> Result<Json<BoardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let board = CardService::get_board(pool, query.board_id.as_deref()).await?;
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

pub async fn get_card_logs(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
) -> Result<Json<Vec<AgentLog>>, KanbanError> {
    let pool = state.require_db()?;
    let logs: Vec<AgentLog> = sqlx::query_as(
        "SELECT * FROM agent_logs WHERE card_id = ? ORDER BY created_at ASC",
    )
    .bind(&card_id)
    .fetch_all(pool)
    .await
    .map_err(|e| KanbanError::Internal(e.to_string()))?;
    Ok(Json(logs))
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

pub async fn generate_plan(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &card_id).await?;

    if card.stage != "plan" {
        return Err(KanbanError::BadRequest(
            "Plan generation is only available for cards in the plan stage".into(),
        ));
    }

    let subtasks = CardService::get_subtasks(pool, &card_id).await?;
    let subtask_titles = if subtasks.is_empty() {
        "None".to_string()
    } else {
        subtasks
            .iter()
            .map(|s| format!("- {}", s.title))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let session_response = state
        .http_client
        .post(format!("{}/session", state.config.opencode_url))
        .json(&json!({}))
        .send()
        .await
        .map_err(|e| {
            KanbanError::OpenCodeError(format!("Failed to create OpenCode session: {}", e))
        })?;

    if !session_response.status().is_success() {
        return Err(KanbanError::OpenCodeError(format!(
            "OpenCode session creation failed with status {}",
            session_response.status()
        )));
    }

    let session_body = session_response
        .json::<Value>()
        .await
        .map_err(|e| KanbanError::OpenCodeError(format!("Failed to decode session response: {}", e)))?;

    let session_id = session_body
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| KanbanError::OpenCodeError("OpenCode session response missing id".into()))?;

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE cards SET ai_session_id = ?, ai_status = ?, updated_at = ? WHERE id = ?")
        .bind(&session_id)
        .bind("planning")
        .bind(&now)
        .bind(&card_id)
        .execute(pool)
        .await?;

    let prompt = format!(
        "You are a project planning assistant. Analyze this card and create a detailed implementation plan.\n\nCard Title: {}\nDescription: {}\nPriority: {}\nWorking Directory: {}\nCurrent Subtasks: {}\n\nYour job:\n1. Analyze the card requirements\n2. Break down the work into concrete, actionable subtasks\n3. For each subtask, use the kanban MCP tool `create_subtask` to add it to the card\n4. Set appropriate phase names for grouping related subtasks\n5. Summarize your plan at the end\n\nUse the kanban MCP tools to create subtasks. The card_id is: {}",
        card.title,
        card.description,
        card.priority,
        card.working_directory,
        subtask_titles,
        card.id,
    );

    let http_client = state.http_client.clone();
    let message_url = format!(
        "{}/session/{}/message",
        state.config.opencode_url,
        session_id.as_str()
    );
    let db_clone = pool.clone();
    let card_id_clone = card_id.clone();

    tokio::spawn(async move {
        let result = http_client
            .post(&message_url)
            .json(&json!({"parts": [{"type": "text", "text": prompt}]}))
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                tracing::info!(card_id = card_id_clone.as_str(), "Plan generation prompt sent successfully");
            }
            Ok(response) => {
                tracing::warn!(
                    card_id = card_id_clone.as_str(),
                    status = %response.status(),
                    "Plan generation message returned non-success"
                );
                let _ = sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                    .bind("failed")
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(&card_id_clone)
                    .execute(&db_clone)
                    .await;
            }
            Err(err) => {
                tracing::warn!(card_id = card_id_clone.as_str(), error = %err, "Failed to send plan generation message");
                let _ = sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                    .bind("failed")
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(&card_id_clone)
                    .execute(&db_clone)
                    .await;
            }
        }
    });

    let updated_card = CardService::get_card_by_id(pool, &card_id).await?;
    Ok(Json(updated_card))
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
