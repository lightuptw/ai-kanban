use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;
use std::path::Path as FsPath;
use std::sync::{Arc, Mutex};

use crate::api::dto::{BoardResponse, CardResponse, CreateCardRequest, MoveCardRequest, UpdateCardRequest};
use crate::api::handlers::sse::WsEvent;
use crate::api::AppState;
use crate::domain::{AgentLog, Card, CardVersion, Comment, KanbanError, Stage};
use crate::services::git_worktree::{ConflictDetail, DiffResult, MergeResult, ResolveRequest};
use crate::services::{AiDispatchService, CardService, GitWorktreeService};

#[derive(Debug, Deserialize)]
pub struct BoardQuery {
    pub board_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct BoardContextRow {
    codebase_path: String,
    context_markdown: String,
    tech_stack: String,
    communication_patterns: String,
    environments: String,
    code_conventions: String,
    testing_requirements: String,
    api_conventions: String,
    infrastructure: String,
    github_repo: String,
}

async fn get_card_codebase_path(pool: &SqlitePool, card_id: &str) -> Result<String, KanbanError> {
    let board_id = sqlx::query_scalar::<_, String>("SELECT board_id FROM cards WHERE id = ?")
        .bind(card_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();

    if board_id.is_empty() {
        return Err(KanbanError::BadRequest("Card is not assigned to a board".into()));
    }

    let codebase_path = sqlx::query_scalar::<_, String>(
        "SELECT codebase_path FROM board_settings WHERE board_id = ?",
    )
    .bind(&board_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_default();

    if codebase_path.is_empty() {
        return Err(KanbanError::BadRequest(
            "Board codebase path not configured".into(),
        ));
    }

    Ok(codebase_path)
}

fn broadcast_event(state: &AppState, event: &WsEvent) {
    if let Ok(payload) = serde_json::to_string(event) {
        let _ = state.sse_tx.send(payload);
    }
}

struct MergeLockGuard {
    locks: Arc<Mutex<HashSet<String>>>,
    codebase_path: String,
    keep_lock: bool,
}

impl MergeLockGuard {
    fn acquire(state: &AppState, codebase_path: &str) -> Result<Self, KanbanError> {
        let mut locks = state
            .merge_locks
            .lock()
            .map_err(|_| KanbanError::Internal("Merge lock state poisoned".into()))?;

        if locks.contains(codebase_path) {
            return Err(KanbanError::Conflict(
                "A merge operation is already active for this codebase".into(),
            ));
        }

        locks.insert(codebase_path.to_string());
        drop(locks);

        Ok(Self {
            locks: Arc::clone(&state.merge_locks),
            codebase_path: codebase_path.to_string(),
            keep_lock: false,
        })
    }

    fn keep_lock(&mut self) {
        self.keep_lock = true;
    }
}

impl Drop for MergeLockGuard {
    fn drop(&mut self) {
        if self.keep_lock {
            return;
        }

        if let Ok(mut locks) = self.locks.lock() {
            locks.remove(&self.codebase_path);
        }
    }
}

fn release_merge_lock(state: &AppState, codebase_path: &str) -> Result<(), KanbanError> {
    let mut locks = state
        .merge_locks
        .lock()
        .map_err(|_| KanbanError::Internal("Merge lock state poisoned".into()))?;
    locks.remove(codebase_path);
    Ok(())
}

pub async fn create_card(
    State(state): State<AppState>,
    Json(req): Json<CreateCardRequest>,
) -> Result<(StatusCode, Json<CardResponse>), KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::create_card(pool, req).await?;

    let event = WsEvent::CardCreated {
        card: serde_json::to_value(&card).unwrap_or_default(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

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
        .map_err(KanbanError::BadRequest)?;

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
    let event = WsEvent::CardUpdated {
        card: serde_json::to_value(&card).unwrap_or_default(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok(Json(card))
}

pub async fn update_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCardRequest>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::update_card(pool, &id, req).await?;

    let event = WsEvent::CardUpdated {
        card: serde_json::to_value(&card).unwrap_or_default(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

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

    if target_stage == Stage::Done && !previous_card.worktree_path.is_empty() {
        let board_id = sqlx::query_scalar::<_, String>("SELECT board_id FROM cards WHERE id = ?")
            .bind(&id)
            .fetch_optional(pool)
            .await?
            .unwrap_or_default();

        if !board_id.is_empty() {
            if let Ok(codebase_path) = sqlx::query_scalar::<_, String>(
                "SELECT codebase_path FROM board_settings WHERE board_id = ?",
            )
            .bind(&board_id)
            .fetch_one(pool)
            .await
            {
                let _ = GitWorktreeService::remove_worktree(
                    &codebase_path,
                    &previous_card.worktree_path,
                    &previous_card.branch_name,
                );

                if let Err(e) = sqlx::query(
                    "UPDATE cards SET branch_name = '', worktree_path = '', working_directory = '.', updated_at = ? WHERE id = ?",
                )
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(&id)
                .execute(pool)
                .await
                {
                    tracing::warn!(error = %e, card_id = %id, "Failed to clear worktree metadata for completed card");
                }
            }
        }
    }

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

        let event = WsEvent::AiStatusChanged {
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

    // Wake up opencode server (it may be sleeping)
    let _ = state
        .http_client
        .get(format!("{}/health", state.config.opencode_url))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

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

    let board_id = sqlx::query_scalar::<_, String>("SELECT board_id FROM cards WHERE id = ?")
        .bind(&card.id)
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();

    let board_context = if !board_id.is_empty() {
        let settings: Option<BoardContextRow> =
            sqlx::query_as(
                "SELECT codebase_path, context_markdown, tech_stack, communication_patterns, environments, code_conventions, testing_requirements, api_conventions, infrastructure, github_repo FROM board_settings WHERE board_id = ?",
            )
            .bind(&board_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

        if let Some(ctx) = settings
        {
            let mut context_sections = Vec::new();

            if !ctx.context_markdown.is_empty() {
                context_sections.push(format!("{}\n\n", ctx.context_markdown));
            }
            if !ctx.codebase_path.trim().is_empty() {
                context_sections.push(format!("### Codebase Path\n{}\n\n", ctx.codebase_path));
            }
            if !ctx.tech_stack.trim().is_empty() {
                context_sections.push(format!("### Tech Stack\n{}\n\n", ctx.tech_stack));
            }
            if !ctx.communication_patterns.trim().is_empty() {
                context_sections.push(format!(
                    "### Communication Patterns\n{}\n\n",
                    ctx.communication_patterns
                ));
            }
            if !ctx.environments.trim().is_empty() {
                context_sections.push(format!("### Environments\n{}\n\n", ctx.environments));
            }
            if !ctx.code_conventions.trim().is_empty() {
                context_sections.push(format!("### Code Conventions\n{}\n\n", ctx.code_conventions));
            }
            if !ctx.testing_requirements.trim().is_empty() {
                context_sections.push(format!(
                    "### Testing Requirements\n{}\n\n",
                    ctx.testing_requirements
                ));
            }
            if !ctx.api_conventions.trim().is_empty() {
                context_sections.push(format!("### API Conventions\n{}\n\n", ctx.api_conventions));
            }
            if !ctx.infrastructure.trim().is_empty() {
                context_sections.push(format!("### Infrastructure\n{}\n\n", ctx.infrastructure));
            }
            if !ctx.github_repo.trim().is_empty() {
                context_sections.push(format!("### GitHub Repository\n{}\n\n", ctx.github_repo));
            }

            context_sections.concat()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

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
## Board Context (apply to ALL work on this board)\n\
{}\n\n\
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
        board_context,
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
                if let Err(e) = sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                    .bind("failed")
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(&card_id_clone)
                    .execute(&db_clone)
                    .await
                {
                    tracing::warn!(error = %e, card_id = card_id_clone.as_str(), "Failed to update card status after plan dispatch failure");
                }
            }
            Err(err) => {
                tracing::warn!(card_id = card_id_clone.as_str(), error = %err, "Failed to send plan generation message");
                if let Err(e) = sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                    .bind("failed")
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(&card_id_clone)
                    .execute(&db_clone)
                    .await
                {
                    tracing::warn!(error = %e, card_id = card_id_clone.as_str(), "Failed to update card status after plan message error");
                }
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

#[derive(Debug, Deserialize)]
pub struct CreatePrRequest {
    pub title: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatePrResponse {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct RejectCardRequest {
    pub feedback: Option<String>,
}

pub async fn get_card_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DiffResult>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    if card.branch_name.is_empty() {
        return Err(KanbanError::BadRequest("Card has no git branch".into()));
    }

    let codebase_path = get_card_codebase_path(pool, &id).await?;
    let diff = GitWorktreeService::get_diff(&codebase_path, &card.branch_name)?;
    Ok(Json(diff))
}

pub async fn get_conflicts(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ConflictDetail>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    if card.stage != "review" {
        return Err(KanbanError::BadRequest(
            "Card must be in review stage to inspect merge conflicts".into(),
        ));
    }

    let codebase_path = get_card_codebase_path(pool, &id).await?;
    if !GitWorktreeService::is_merge_in_progress(&codebase_path) {
        return Err(KanbanError::BadRequest(
            "No merge in progress for this card".into(),
        ));
    }

    let detail = GitWorktreeService::get_conflict_details(&codebase_path)?;
    Ok(Json(detail))
}

pub async fn resolve_conflicts(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ResolveRequest>,
) -> Result<Json<ConflictDetail>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    if card.stage != "review" {
        return Err(KanbanError::BadRequest(
            "Card must be in review stage to resolve conflicts".into(),
        ));
    }

    let codebase_path = get_card_codebase_path(pool, &id).await?;
    if !GitWorktreeService::is_merge_in_progress(&codebase_path) {
        return Err(KanbanError::BadRequest(
            "No merge in progress for this card".into(),
        ));
    }

    for resolution in &req.resolutions {
        match resolution.choice.as_str() {
            "ours" => {
                GitWorktreeService::run_git(
                    &codebase_path,
                    &["checkout", "--ours", "--", resolution.file_path.as_str()],
                )?;
                GitWorktreeService::run_git(
                    &codebase_path,
                    &["add", "--", resolution.file_path.as_str()],
                )?;
            }
            "theirs" => {
                GitWorktreeService::run_git(
                    &codebase_path,
                    &["checkout", "--theirs", "--", resolution.file_path.as_str()],
                )?;
                GitWorktreeService::run_git(
                    &codebase_path,
                    &["add", "--", resolution.file_path.as_str()],
                )?;
            }
            "manual" => {
                let manual_content = resolution.manual_content.as_ref().ok_or_else(|| {
                    KanbanError::BadRequest(format!(
                        "manual_content is required for manual resolution: {}",
                        resolution.file_path
                    ))
                })?;

                let file_path = FsPath::new(&codebase_path).join(&resolution.file_path);
                std::fs::write(&file_path, manual_content).map_err(|e| {
                    KanbanError::Internal(format!(
                        "Failed to write manual resolution for {}: {}",
                        resolution.file_path, e
                    ))
                })?;

                GitWorktreeService::run_git(
                    &codebase_path,
                    &["add", "--", resolution.file_path.as_str()],
                )?;
            }
            _ => {
                return Err(KanbanError::BadRequest(format!(
                    "Invalid resolution choice '{}' for file {}",
                    resolution.choice, resolution.file_path
                )));
            }
        }
    }

    let detail = GitWorktreeService::get_conflict_details(&codebase_path)?;

    let event = WsEvent::MergeConflictResolved {
        card_id: id,
        remaining_count: detail.files.len(),
    };
    broadcast_event(&state, &event);

    Ok(Json(detail))
}

pub async fn complete_merge(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MergeResult>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    if card.stage != "review" {
        return Err(KanbanError::BadRequest(
            "Card must be in review stage to complete merge".into(),
        ));
    }
    if card.branch_name.is_empty() {
        return Err(KanbanError::BadRequest("Card has no git branch".into()));
    }

    let codebase_path = get_card_codebase_path(pool, &id).await?;
    if !GitWorktreeService::is_merge_in_progress(&codebase_path) {
        return Err(KanbanError::BadRequest(
            "No merge in progress for this card".into(),
        ));
    }

    let unmerged = GitWorktreeService::run_git(&codebase_path, &["ls-files", "--unmerged"])?;
    if !unmerged.trim().is_empty() {
        return Err(KanbanError::BadRequest(
            "Cannot complete merge while conflicts remain".into(),
        ));
    }

    GitWorktreeService::run_git(&codebase_path, &["commit", "--no-edit"])?;
    if let Err(error) = GitWorktreeService::run_git(&codebase_path, &["checkout", "-"]) {
        tracing::warn!(error = %error, card_id = %id, "Failed to return to previous branch after merge completion");
    }

    let _ = GitWorktreeService::remove_worktree(&codebase_path, &card.worktree_path, &card.branch_name);

    sqlx::query("UPDATE cards SET stage = 'done', branch_name = '', worktree_path = '', working_directory = '.', updated_at = ? WHERE id = ?")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&id)
        .execute(pool)
        .await?;

    release_merge_lock(&state, &codebase_path)?;

    broadcast_event(&state, &WsEvent::MergeCompleted { card_id: id.clone() });
    broadcast_event(
        &state,
        &WsEvent::CardMoved {
            card_id: id.clone(),
            from_stage: "review".to_string(),
            to_stage: "done".to_string(),
        },
    );

    Ok(Json(MergeResult {
        success: true,
        message: "Merge completed successfully".to_string(),
        conflicts: Vec::new(),
        conflict_detail: None,
    }))
}

pub async fn abort_merge(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    if card.stage != "review" {
        return Err(KanbanError::BadRequest(
            "Card must be in review stage to abort merge".into(),
        ));
    }

    let codebase_path = get_card_codebase_path(pool, &id).await?;

    if let Err(abort_error) = GitWorktreeService::run_git(&codebase_path, &["merge", "--abort"]) {
        tracing::warn!(
            error = %abort_error,
            card_id = %id,
            "git merge --abort failed, using fallback reset"
        );
        GitWorktreeService::run_git(&codebase_path, &["reset", "--hard", "HEAD"])?;
    }

    if let Err(checkout_error) = GitWorktreeService::run_git(&codebase_path, &["checkout", "-"]) {
        tracing::warn!(
            error = %checkout_error,
            card_id = %id,
            "Failed to return to previous branch after abort"
        );
    }

    release_merge_lock(&state, &codebase_path)?;

    broadcast_event(&state, &WsEvent::MergeAborted { card_id: id });

    Ok(StatusCode::OK)
}

pub async fn merge_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MergeResult>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    if card.branch_name.is_empty() {
        return Err(KanbanError::BadRequest("Card has no git branch".into()));
    }
    if card.stage != "review" {
        return Err(KanbanError::BadRequest(
            "Card must be in review stage to merge".into(),
        ));
    }

    let codebase_path = get_card_codebase_path(pool, &id).await?;
    let mut merge_lock = MergeLockGuard::acquire(&state, &codebase_path)?;
    let result = GitWorktreeService::merge_branch(&codebase_path, &card.branch_name, true)?;

    if result.success {
        let _ = GitWorktreeService::remove_worktree(&codebase_path, &card.worktree_path, &card.branch_name);

        sqlx::query("UPDATE cards SET stage = 'done', branch_name = '', worktree_path = '', working_directory = '.', updated_at = ? WHERE id = ?")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(&id)
            .execute(pool)
            .await?;

        broadcast_event(&state, &WsEvent::MergeCompleted { card_id: id.clone() });

        let event = WsEvent::CardMoved {
            card_id: id.clone(),
            from_stage: "review".to_string(),
            to_stage: "done".to_string(),
        };
        broadcast_event(&state, &event);
    } else {
        merge_lock.keep_lock();
        broadcast_event(
            &state,
            &WsEvent::MergeConflictDetected {
                card_id: id.clone(),
                conflict_count: result.conflicts.len(),
            },
        );
    }

    Ok(Json(result))
}

pub async fn create_card_pr(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CreatePrRequest>,
) -> Result<Json<CreatePrResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    if card.branch_name.is_empty() {
        return Err(KanbanError::BadRequest("Card has no git branch".into()));
    }

    let codebase_path = get_card_codebase_path(pool, &id).await?;

    let title = req.title.unwrap_or_else(|| card.title.clone());
    let body = req.body.unwrap_or_else(|| {
        format!(
            "AI-generated changes for card: {}\n\n{}",
            card.title, card.description
        )
    });

    let url = GitWorktreeService::create_github_pr(&codebase_path, &card.branch_name, &title, &body)?;
    Ok(Json(CreatePrResponse { url }))
}

pub async fn reject_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RejectCardRequest>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    if card.stage != "review" {
        return Err(KanbanError::BadRequest(
            "Card must be in review stage to reject".into(),
        ));
    }

    sqlx::query("UPDATE cards SET stage = 'in_progress', ai_status = 'idle', updated_at = ? WHERE id = ?")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&id)
        .execute(pool)
        .await?;

    if let Some(feedback) = &req.feedback {
        if !feedback.is_empty() {
            let comment_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO comments (id, card_id, author, content, created_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&comment_id)
            .bind(&id)
            .bind("Reviewer")
            .bind(format!("**Review Feedback:** {}", feedback))
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(pool)
            .await?;
        }
    }

    let event = WsEvent::CardMoved {
        card_id: id.clone(),
        from_stage: "review".to_string(),
        to_stage: "in_progress".to_string(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    let updated = CardService::get_card_by_id(pool, &id).await?;
    Ok(Json(updated))
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

    if !card.worktree_path.is_empty() {
        if let Ok(codebase_path) = get_card_codebase_path(pool, &id).await {
            let _ = GitWorktreeService::remove_worktree(
                &codebase_path,
                &card.worktree_path,
                &card.branch_name,
            );
            if let Err(e) = sqlx::query(
                "UPDATE cards SET branch_name = '', worktree_path = '', working_directory = '.', updated_at = ? WHERE id = ?",
            )
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(&id)
            .execute(pool)
            .await
            {
                tracing::warn!(error = %e, card_id = %id, "Failed to clear worktree metadata after AI stop");
            }
        }
    }

    let event = WsEvent::AiStatusChanged {
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

pub async fn resume_ai(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CardResponse>, KanbanError> {
    let pool = state.require_db()?;
    let card = CardService::get_card_model(pool, &id).await?;

    let valid_stages = ["plan", "todo", "in_progress"];
    if !valid_stages.contains(&card.stage.as_str()) {
        return Err(KanbanError::BadRequest(
            "AI resume is only available for cards in plan, todo, or in_progress stage".into(),
        ));
    }

    let active_statuses = ["planning", "dispatched", "working", "queued", "waiting_input"];
    if active_statuses.contains(&card.ai_status.as_str()) {
        return Err(KanbanError::BadRequest(format!(
            "Card AI is already active with status '{}'",
            card.ai_status
        )));
    }

    if let Some(session_id) = card.ai_session_id.as_deref().filter(|s| !s.is_empty()) {
        let _ = state
            .http_client
            .get(format!("{}/health", state.config.opencode_url))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;

        let session_check_url = format!("{}/session/{}", state.config.opencode_url, session_id);
        let session_exists = match state
            .http_client
            .get(&session_check_url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => true,
            Ok(response) => {
                tracing::warn!(
                    card_id = id.as_str(),
                    session_id,
                    status = %response.status(),
                    "Stored AI session no longer available; falling back to fresh dispatch"
                );
                false
            }
            Err(err) => {
                tracing::warn!(
                    card_id = id.as_str(),
                    session_id,
                    error = %err,
                    "Failed to verify existing AI session; falling back to fresh dispatch"
                );
                false
            }
        };

        if session_exists {
            let resumed_status = if card.stage == "plan" {
                "planning"
            } else {
                "working"
            };
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                .bind(resumed_status)
                .bind(&now)
                .bind(&id)
                .execute(pool)
                .await?;

            let event = WsEvent::AiStatusChanged {
                card_id: id.clone(),
                status: resumed_status.to_string(),
                progress: json!({}),
                stage: card.stage.clone(),
                ai_session_id: card.ai_session_id.clone(),
            };
            let _ = state.sse_tx.send(serde_json::to_string(&event).unwrap_or_default());

            let prompt = if card.stage == "plan" {
                "Continue planning this card. Review what subtasks already exist and create any remaining ones. Add a summary comment when done.".to_string()
            } else {
                "Continue where you left off. Review which subtasks are completed vs pending, then resume work on the remaining items.".to_string()
            };

            let http_client = state.http_client.clone();
            let message_url = format!(
                "{}/session/{}/message",
                state.config.opencode_url,
                session_id
            );
            let db_clone = pool.clone();
            let card_id_clone = id.clone();

            tokio::spawn(async move {
                let result = http_client
                    .post(&message_url)
                    .json(&json!({"parts": [{"type": "text", "text": prompt}]}))
                    .send()
                    .await;

                match result {
                    Ok(response) if response.status().is_success() => {
                        tracing::info!(card_id = card_id_clone.as_str(), "Resume message sent successfully");
                    }
                    Ok(response) => {
                        tracing::warn!(
                            card_id = card_id_clone.as_str(),
                            status = %response.status(),
                            "Resume message returned non-success"
                        );
                        if let Err(e) = sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                            .bind("failed")
                            .bind(chrono::Utc::now().to_rfc3339())
                            .bind(&card_id_clone)
                            .execute(&db_clone)
                            .await
                        {
                            tracing::warn!(error = %e, card_id = card_id_clone.as_str(), "Failed to update card status after resume non-success response");
                        }
                    }
                    Err(err) => {
                        tracing::warn!(card_id = card_id_clone.as_str(), error = %err, "Failed to send resume message");
                        if let Err(e) = sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                            .bind("failed")
                            .bind(chrono::Utc::now().to_rfc3339())
                            .bind(&card_id_clone)
                            .execute(&db_clone)
                            .await
                        {
                            tracing::warn!(error = %e, card_id = card_id_clone.as_str(), "Failed to update card status after resume message error");
                        }
                    }
                }
            });

            let updated = CardService::get_card_by_id(pool, &id).await?;
            return Ok(Json(updated));
        }
    }

    let (fallback_status, fallback_session_id) = if card.stage == "plan" {
        tracing::info!(
            card_id = id.as_str(),
            "Resume fallback for plan card: resetting to idle with no session"
        );
        ("idle", None::<String>)
    } else {
        tracing::info!(
            card_id = id.as_str(),
            "Resume fallback for execution card: queueing with no session"
        );
        ("queued", None::<String>)
    };

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE cards SET ai_session_id = ?, ai_status = ?, updated_at = ? WHERE id = ?")
        .bind(fallback_session_id)
        .bind(fallback_status)
        .bind(&now)
        .bind(&id)
        .execute(pool)
        .await?;

    let event = WsEvent::AiStatusChanged {
        card_id: id.clone(),
        status: fallback_status.to_string(),
        progress: json!({}),
        stage: card.stage.clone(),
        ai_session_id: None,
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

    if let Ok(card) = CardService::get_card_model(pool, &id).await {
        if !card.worktree_path.is_empty() {
            let board_id = sqlx::query_scalar::<_, String>("SELECT board_id FROM cards WHERE id = ?")
                .bind(&id)
                .fetch_optional(pool)
                .await?
                .unwrap_or_default();

            if !board_id.is_empty() {
                if let Ok(codebase_path) = sqlx::query_scalar::<_, String>(
                    "SELECT codebase_path FROM board_settings WHERE board_id = ?",
                )
                .bind(&board_id)
                .fetch_one(pool)
                .await
                {
                    let _ = GitWorktreeService::remove_worktree(
                        &codebase_path,
                        &card.worktree_path,
                        &card.branch_name,
                    );
                }
            }
        }
    }

    CardService::delete_card(pool, &id).await?;

    let event = WsEvent::CardDeleted {
        card_id: id.clone(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok(StatusCode::NO_CONTENT)
}
