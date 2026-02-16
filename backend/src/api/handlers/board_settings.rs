use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::handlers::sse::SseEvent;
use crate::api::state::AppState;
use crate::domain::KanbanError;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BoardSettings {
    pub board_id: String,
    pub codebase_path: String,
    pub github_repo: String,
    pub context_markdown: String,
    pub document_links: String,
    pub variables: String,
    pub tech_stack: String,
    pub communication_patterns: String,
    pub environments: String,
    pub code_conventions: String,
    pub testing_requirements: String,
    pub api_conventions: String,
    pub infrastructure: String,
    pub ai_concurrency: i64,
    pub auto_detect_status: String,
    pub auto_detect_session_id: String,
    pub auto_detect_started_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBoardSettingsRequest {
    pub codebase_path: Option<String>,
    pub github_repo: Option<String>,
    pub context_markdown: Option<String>,
    pub document_links: Option<String>,
    pub variables: Option<String>,
    pub tech_stack: Option<String>,
    pub communication_patterns: Option<String>,
    pub environments: Option<String>,
    pub code_conventions: Option<String>,
    pub testing_requirements: Option<String>,
    pub api_conventions: Option<String>,
    pub infrastructure: Option<String>,
    pub ai_concurrency: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AutoDetectBoardSettingsRequest {
    pub codebase_path: String,
}

#[derive(Debug, Serialize)]
pub struct AutoDetectBoardSettingsResponse {
    pub status: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CloneRepoRequest {
    pub github_url: String,
    pub clone_path: String,
    pub pat: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CloneRepoResponse {
    pub success: bool,
    pub codebase_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AutoDetectStatusResponse {
    pub status: String,
    pub session_id: String,
    pub started_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AutoDetectLogsQuery {
    pub session_id: String,
}

pub async fn get_board_settings(
    State(state): State<AppState>,
    Path(board_id): Path<String>,
) -> Result<Json<BoardSettings>, KanbanError> {
    let pool = state.require_db()?;

    let settings: Option<BoardSettings> = sqlx::query_as(
        "SELECT board_id, codebase_path, github_repo, context_markdown, document_links, variables, tech_stack, communication_patterns, environments, code_conventions, testing_requirements, api_conventions, infrastructure, ai_concurrency, auto_detect_status, auto_detect_session_id, auto_detect_started_at, created_at, updated_at FROM board_settings WHERE board_id = ?",
    )
    .bind(&board_id)
    .fetch_optional(pool)
    .await?;

    Ok(Json(settings.unwrap_or(BoardSettings {
        board_id,
        codebase_path: String::new(),
        github_repo: String::new(),
        context_markdown: String::new(),
        document_links: "[]".to_string(),
        variables: "{}".to_string(),
        tech_stack: String::new(),
        communication_patterns: String::new(),
        environments: String::new(),
        code_conventions: String::new(),
        testing_requirements: String::new(),
        api_conventions: String::new(),
        infrastructure: String::new(),
        ai_concurrency: 1,
        auto_detect_status: String::new(),
        auto_detect_session_id: String::new(),
        auto_detect_started_at: String::new(),
        created_at: String::new(),
        updated_at: String::new(),
    })))
}

pub async fn update_board_settings(
    State(state): State<AppState>,
    Path(board_id): Path<String>,
    Json(req): Json<UpdateBoardSettingsRequest>,
) -> Result<Json<BoardSettings>, KanbanError> {
    let pool = state.require_db()?;
    let now = chrono::Utc::now().to_rfc3339();

    // Fetch existing settings to merge with partial update
    let existing: Option<BoardSettings> = sqlx::query_as(
        "SELECT board_id, codebase_path, github_repo, context_markdown, document_links, variables, tech_stack, communication_patterns, environments, code_conventions, testing_requirements, api_conventions, infrastructure, ai_concurrency, auto_detect_status, auto_detect_session_id, auto_detect_started_at, created_at, updated_at FROM board_settings WHERE board_id = ?",
    )
    .bind(&board_id)
    .fetch_optional(pool)
    .await?;

    let (cb, gr, cm, dl, va, ts, cp, en, cc, tr, ac, inf, aic) = match &existing {
        Some(e) => (
            req.codebase_path.unwrap_or_else(|| e.codebase_path.clone()),
            req.github_repo.unwrap_or_else(|| e.github_repo.clone()),
            req.context_markdown.unwrap_or_else(|| e.context_markdown.clone()),
            req.document_links.unwrap_or_else(|| e.document_links.clone()),
            req.variables.unwrap_or_else(|| e.variables.clone()),
            req.tech_stack.unwrap_or_else(|| e.tech_stack.clone()),
            req.communication_patterns.unwrap_or_else(|| e.communication_patterns.clone()),
            req.environments.unwrap_or_else(|| e.environments.clone()),
            req.code_conventions.unwrap_or_else(|| e.code_conventions.clone()),
            req.testing_requirements.unwrap_or_else(|| e.testing_requirements.clone()),
            req.api_conventions.unwrap_or_else(|| e.api_conventions.clone()),
            req.infrastructure.unwrap_or_else(|| e.infrastructure.clone()),
            req.ai_concurrency.unwrap_or(e.ai_concurrency),
        ),
        None => (
            req.codebase_path.unwrap_or_default(),
            req.github_repo.unwrap_or_default(),
            req.context_markdown.unwrap_or_default(),
            req.document_links.unwrap_or_else(|| "[]".to_string()),
            req.variables.unwrap_or_else(|| "{}".to_string()),
            req.tech_stack.unwrap_or_default(),
            req.communication_patterns.unwrap_or_default(),
            req.environments.unwrap_or_default(),
            req.code_conventions.unwrap_or_default(),
            req.testing_requirements.unwrap_or_default(),
            req.api_conventions.unwrap_or_default(),
            req.infrastructure.unwrap_or_default(),
            req.ai_concurrency.unwrap_or(1),
        ),
    };

    let settings: BoardSettings = sqlx::query_as(
        "INSERT INTO board_settings (board_id, codebase_path, github_repo, context_markdown, document_links, variables, tech_stack, communication_patterns, environments, code_conventions, testing_requirements, api_conventions, infrastructure, ai_concurrency, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(board_id) DO UPDATE SET
             codebase_path = excluded.codebase_path,
             github_repo = excluded.github_repo,
             context_markdown = excluded.context_markdown,
             document_links = excluded.document_links,
             variables = excluded.variables,
             tech_stack = excluded.tech_stack,
             communication_patterns = excluded.communication_patterns,
             environments = excluded.environments,
             code_conventions = excluded.code_conventions,
             testing_requirements = excluded.testing_requirements,
             api_conventions = excluded.api_conventions,
              infrastructure = excluded.infrastructure,
              ai_concurrency = excluded.ai_concurrency,
              updated_at = excluded.updated_at
         RETURNING board_id, codebase_path, github_repo, context_markdown, document_links, variables, tech_stack, communication_patterns, environments, code_conventions, testing_requirements, api_conventions, infrastructure, ai_concurrency, auto_detect_status, auto_detect_session_id, auto_detect_started_at, created_at, updated_at",
    )
    .bind(&board_id)
    .bind(&cb)
    .bind(&gr)
    .bind(&cm)
    .bind(&dl)
    .bind(&va)
    .bind(&ts)
    .bind(&cp)
    .bind(&en)
    .bind(&cc)
    .bind(&tr)
    .bind(&ac)
    .bind(&inf)
    .bind(aic)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;

    Ok(Json(settings))
}

pub async fn auto_detect_board_settings(
    State(state): State<AppState>,
    Path(board_id): Path<String>,
    Json(req): Json<AutoDetectBoardSettingsRequest>,
) -> Result<(StatusCode, Json<AutoDetectBoardSettingsResponse>), KanbanError> {
    let pool = state.require_db()?;
    let codebase_path = req.codebase_path;

    if !std::path::Path::new(&codebase_path).exists() {
        return Err(KanbanError::BadRequest(format!(
            "Codebase path does not exist: {}",
            codebase_path
        )));
    }

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
        .json::<serde_json::Value>()
        .await
        .map_err(|e| KanbanError::OpenCodeError(format!("Failed to decode session response: {}", e)))?;

    let session_id = session_body
        .get("id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| KanbanError::OpenCodeError("Session response missing id".into()))?;

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO board_settings (board_id, codebase_path, created_at, updated_at, auto_detect_status, auto_detect_session_id, auto_detect_started_at)
         VALUES (?, ?, ?, ?, 'running', ?, ?)
         ON CONFLICT(board_id) DO UPDATE SET
             codebase_path = excluded.codebase_path,
             auto_detect_status = 'running',
             auto_detect_session_id = excluded.auto_detect_session_id,
             auto_detect_started_at = excluded.auto_detect_started_at,
             updated_at = excluded.updated_at",
    )
    .bind(&board_id)
    .bind(&codebase_path)
    .bind(&now)
    .bind(&now)
    .bind(&session_id)
    .bind(&now)
    .execute(pool)
    .await?;

    let event = SseEvent::AutoDetectStatus {
        board_id: board_id.clone(),
        status: "running".to_string(),
        session_id: Some(session_id.clone()),
        elapsed_seconds: Some(0),
        message: Some("AI is analyzing your codebase...".to_string()),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    let prompt = format!(
        r#"You are a codebase analysis assistant. Analyze the codebase at "{codebase_path}" and fill in the board settings.

Board ID: {board_id}

## Instructions
1. Explore the codebase directory structure, config files, package manifests, and source code
2. Identify: tech stack, frameworks, languages, versions
3. Identify: communication patterns (REST, gRPC, WebSocket, message queues)
4. Identify: environments (dev, staging, production) from config files
5. Identify: code conventions (linter configs, formatting, naming patterns)
6. Identify: testing frameworks and requirements
7. Identify: API conventions (REST style, auth patterns, error formats)
8. Identify: infrastructure (Docker, K8s, CI/CD, cloud provider)
9. Use the `kanban_update_board_settings` MCP tool to save ALL findings to board_id "{board_id}"
10. Set each field with a clear, structured summary (use bullet points, be specific with version numbers)

## SAFETY RULES
- ONLY use kanban MCP tools to save results
- Do NOT modify any files in the codebase
- Do NOT run any build or test commands
- Do NOT access databases directly
- READ ONLY -- analyze files, do not change them"#,
        codebase_path = codebase_path,
        board_id = board_id,
    );

    let http_client = state.http_client.clone();
    let message_url = format!("{}/session/{}/message", state.config.opencode_url, &session_id);
    let db_clone = pool.clone();
    let board_id_clone = board_id.clone();
    let sse_tx_clone = state.sse_tx.clone();
    let started_at = chrono::Utc::now();

    tokio::spawn(async move {
        let result = http_client
            .post(&message_url)
            .json(&json!({"parts": [{"type": "text", "text": prompt}]}))
            .send()
            .await;

        let (status, message) = match result {
            Ok(response) if response.status().is_success() => {
                tracing::info!(board_id = board_id_clone.as_str(), "Auto-detect prompt sent successfully");
                ("completed".to_string(), "Analysis complete!".to_string())
            }
            Ok(response) => {
                tracing::warn!(board_id = board_id_clone.as_str(), status = %response.status(), "Auto-detect returned non-success");
                (
                    "failed".to_string(),
                    format!("Analysis failed with status {}", response.status()),
                )
            }
            Err(err) => {
                tracing::warn!(board_id = board_id_clone.as_str(), error = %err, "Failed to send auto-detect message");
                (
                    "failed".to_string(),
                    format!("Failed to send analysis: {}", err),
                )
            }
        };

        let elapsed = (chrono::Utc::now() - started_at).num_seconds().max(0) as u64;
        let now = chrono::Utc::now().to_rfc3339();

        if let Err(e) = sqlx::query(
            "UPDATE board_settings SET auto_detect_status = ?, updated_at = ? WHERE board_id = ?",
        )
        .bind(&status)
        .bind(&now)
        .bind(&board_id_clone)
        .execute(&db_clone)
        .await
        {
            tracing::warn!(error = %e, board_id = board_id_clone.as_str(), "Failed to persist auto-detect status update");
        }

        let event = SseEvent::AutoDetectStatus {
            board_id: board_id_clone,
            status,
            session_id: None,
            elapsed_seconds: Some(elapsed),
            message: Some(message),
        };
        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = sse_tx_clone.send(payload);
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(AutoDetectBoardSettingsResponse {
            status: "running".to_string(),
            session_id: Some(session_id),
        }),
    ))
}

pub async fn clone_repo(
    State(state): State<AppState>,
    Path(board_id): Path<String>,
    Json(req): Json<CloneRepoRequest>,
) -> Result<Json<CloneRepoResponse>, KanbanError> {
    let pool = state.require_db()?;

    let parent = std::path::Path::new(&req.clone_path)
        .parent()
        .ok_or_else(|| KanbanError::BadRequest("Invalid clone path".into()))?;
    if !parent.exists() {
        return Err(KanbanError::BadRequest(format!(
            "Parent directory does not exist: {}",
            parent.display()
        )));
    }

    let clone_url = if let Some(pat) = &req.pat {
        req.github_url
            .replace("https://", &format!("https://{}@", pat))
    } else {
        req.github_url.clone()
    };

    let output = std::process::Command::new("git")
        .args(["clone", &clone_url, &req.clone_path])
        .output()
        .map_err(|e| KanbanError::Internal(format!("Failed to run git clone: {}", e)))?;

    if output.status.success() {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO board_settings (board_id, codebase_path, created_at, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(board_id) DO UPDATE SET codebase_path = excluded.codebase_path, updated_at = excluded.updated_at",
        )
        .bind(&board_id)
        .bind(&req.clone_path)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(Json(CloneRepoResponse {
            success: true,
            codebase_path: Some(req.clone_path),
            error: None,
        }))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let error = if stderr.contains("Authentication")
            || stderr.contains("403")
            || stderr.contains("fatal: could not read Username")
        {
            "auth_required".to_string()
        } else {
            stderr
        };
        Ok(Json(CloneRepoResponse {
            success: false,
            codebase_path: None,
            error: Some(error),
        }))
    }
}

pub async fn get_auto_detect_status(
    State(state): State<AppState>,
    Path(board_id): Path<String>,
) -> Result<Json<AutoDetectStatusResponse>, KanbanError> {
    let pool = state.require_db()?;

    let result: Option<(String, String, String)> = sqlx::query_as(
        "SELECT auto_detect_status, auto_detect_session_id, auto_detect_started_at FROM board_settings WHERE board_id = ?",
    )
    .bind(&board_id)
    .fetch_optional(pool)
    .await?;

    let (status, session_id, started_at) = result.unwrap_or_default();

    Ok(Json(AutoDetectStatusResponse {
        status,
        session_id,
        started_at,
    }))
}

pub async fn get_auto_detect_logs(
    State(state): State<AppState>,
    Path(_board_id): Path<String>,
    Query(query): Query<AutoDetectLogsQuery>,
) -> Result<Json<serde_json::Value>, KanbanError> {
    let url = format!("{}/session/{}", state.config.opencode_url, query.session_id);

    let response = state
        .http_client
        .get(&url)
        .send()
        .await
        .map_err(|e| KanbanError::OpenCodeError(format!("Failed to fetch session: {}", e)))?;

    let body = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| KanbanError::OpenCodeError(format!("Failed to parse session: {}", e)))?;

    Ok(Json(body))
}
