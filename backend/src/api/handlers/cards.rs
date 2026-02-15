use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::api::dto::{BoardResponse, CardResponse, CreateCardRequest, MoveCardRequest, UpdateCardRequest};
use crate::api::handlers::sse::SseEvent;
use crate::api::AppState;
use crate::domain::{AgentLog, Card, CardVersion, Comment, KanbanError, Stage};
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

pub async fn list_card_versions(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
) -> Result<Json<Vec<CardVersion>>, KanbanError> {
    let pool = state.require_db()?;
    let versions: Vec<CardVersion> = sqlx::query_as(
        "SELECT * FROM card_versions WHERE card_id = ? ORDER BY created_at DESC LIMIT 50",
    )
    .bind(&card_id)
    .fetch_all(pool)
    .await?;

    Ok(Json(versions))
}

pub async fn restore_card_version(
    State(state): State<AppState>,
    Path((card_id, version_id)): Path<(String, String)>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let version: CardVersion =
        sqlx::query_as("SELECT * FROM card_versions WHERE id = ? AND card_id = ?")
            .bind(&version_id)
            .bind(&card_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                KanbanError::NotFound(format!(
                    "Card version not found: {} for card {}",
                    version_id, card_id
                ))
            })?;

    let snapshot: serde_json::Value = serde_json::from_str(&version.snapshot)
        .map_err(|e| KanbanError::BadRequest(format!("Invalid card snapshot payload: {}", e)))?;

    let current_card = CardService::get_card_model(pool, &card_id).await?;
    CardService::save_card_version_snapshot(pool, &current_card, "restore").await?;

    let title = snapshot
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let description = snapshot
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let priority = snapshot
        .get("priority")
        .and_then(Value::as_str)
        .unwrap_or("medium")
        .to_string();
    let stage = snapshot
        .get("stage")
        .and_then(Value::as_str)
        .unwrap_or(current_card.stage.as_str())
        .to_string();
    let working_directory = snapshot
        .get("working_directory")
        .and_then(Value::as_str)
        .unwrap_or(".")
        .to_string();
    let linked_documents = snapshot
        .get("linked_documents")
        .and_then(Value::as_str)
        .unwrap_or("[]")
        .to_string();

    stage
        .parse::<Stage>()
        .map_err(|e| KanbanError::BadRequest(e))?;

    sqlx::query(
        "UPDATE cards SET title = ?, description = ?, stage = ?, priority = ?, working_directory = ?, linked_documents = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&title)
    .bind(&description)
    .bind(&stage)
    .bind(&priority)
    .bind(&working_directory)
    .bind(&linked_documents)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(&card_id)
    .execute(pool)
    .await?;

    let card = CardService::get_card_by_id(pool, &card_id).await?;
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
            sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                .bind("queued")
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(&id)
                .execute(pool)
                .await?;
        }

        let updated_card = CardService::get_card_by_id(pool, &id).await?;

        let event = SseEvent::AiStatusChanged {
            card_id: id,
            status: updated_card.ai_status.clone(),
            progress: updated_card.ai_progress.clone(),
            stage: updated_card.stage.clone(),
            ai_session_id: updated_card.ai_session_id.clone(),
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

    let linked_docs_formatted = serde_json::from_str::<Vec<String>>(&card.linked_documents)
        .ok()
        .filter(|docs| !docs.is_empty())
        .map(|docs| {
            docs.iter()
                .map(|doc| format!("- {}", doc))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "None".to_string());

    let attached_files_formatted = match sqlx::query(
        "SELECT original_filename, file_size, mime_type FROM card_files WHERE card_id = ? ORDER BY uploaded_at ASC",
    )
    .bind(&card_id)
    .fetch_all(pool)
    .await
    {
        Ok(rows) if rows.is_empty() => "None".to_string(),
        Ok(rows) => rows
            .iter()
            .map(|row| {
                let name: String = row.get("original_filename");
                let size: i64 = row.get("file_size");
                let mime: String = row.get("mime_type");
                format!("- {} ({} bytes, {})", name, size, mime)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Err(_) => "None".to_string(),
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
        "IMPORTANT: You are working on card_id = \"{}\". ALL subtasks must be created on THIS card. Do NOT create new cards.\n\n\
## SAFETY RULES — MANDATORY\n\
- ONLY use the provided kanban MCP tools: kanban_create_subtask, kanban_update_card, kanban_add_comment, kanban_get_card\n\
- Do NOT search the filesystem for database files\n\
- Do NOT create, open, or modify any .db or .sqlite files\n\
- Do NOT use Python, sqlite3, shell commands, or any tool to access databases directly\n\
- Do NOT attempt to fix MCP tool errors by accessing underlying infrastructure\n\
- If a kanban MCP tool returns an error, STOP and report the error. Do NOT work around it.\n\n\
You are a project planning assistant. Analyze this card and create a detailed implementation plan.\n\n\
## Card Details\n\
- Card ID: {}\n\
- Title: {}\n\
- Description: {}\n\
- Priority: {}\n\
- Working Directory: {}\n\
- Linked Documents:\n{}\n\
- Attached Files:\n{}\n\
- Current Subtasks:\n{}\n\n\
## Instructions\n\
1. Analyze the card requirements\n\
2. Break down the work into concrete, actionable subtasks organized by phases\n\
3. Use the `kanban_create_subtask` MCP tool to add each subtask to card_id \"{}\"\n\
4. Set appropriate phase names (e.g., \"Design\", \"Implementation\", \"Testing\") and phase_order for grouping\n\
5. If you create any plan documents or markdown files, update the card's linked_documents using `kanban_update_card`\n\
6. Add a summary comment using `kanban_add_comment`\n\n\
CRITICAL: The card_id for ALL tool calls is: {}",
        card.id,
        card.id,
        card.title,
        card.description,
        card.priority,
        card.working_directory,
        linked_docs_formatted,
        attached_files_formatted,
        subtask_titles,
        card.id,
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
    _state: &AppState,
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

    sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
        .bind("queued")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&card.id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn stop_ai(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    let session_id = card
        .ai_session_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| KanbanError::Internal("No active AI session on this card".into()))?;

    let active_statuses = ["planning", "dispatched", "working", "queued"];
    if !active_statuses.contains(&card.ai_status.as_str()) {
        return Err(KanbanError::Internal(format!(
            "Card AI status is '{}', not active",
            card.ai_status
        )));
    }

    let dispatch = AiDispatchService::new(
        state.http_client.clone(),
        state.config.opencode_url.clone(),
    );
    if let Err(e) = dispatch.abort_session(session_id).await {
        tracing::warn!(card_id = id.as_str(), error = %e, "Failed to abort opencode session, marking cancelled anyway");
    }

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
        .bind("cancelled")
        .bind(&now)
        .bind(&id)
        .execute(pool)
        .await?;

    let event = SseEvent::AiStatusChanged {
        card_id: id.clone(),
        status: "cancelled".to_string(),
        progress: json!({}),
        stage: card.stage.clone(),
        ai_session_id: card.ai_session_id.clone(),
    };
    let _ = state.sse_tx.send(serde_json::to_string(&event).unwrap_or_default());

    let updated = CardService::get_card_by_id(pool, &id).await?;
    Ok(Json(updated))
}

pub async fn delete_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    CardService::delete_card(pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
