use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Deserialize)]
pub struct AutoDetectBoardSettingsRequest {
    pub codebase_path: String,
}

#[derive(Debug, Serialize)]
pub struct AutoDetectBoardSettingsResponse {
    pub status: String,
}

pub async fn get_board_settings(
    State(state): State<AppState>,
    Path(board_id): Path<String>,
) -> Result<Json<BoardSettings>, KanbanError> {
    let pool = state.require_db()?;

    let settings: Option<BoardSettings> = sqlx::query_as(
        "SELECT board_id, codebase_path, github_repo, context_markdown, document_links, variables, tech_stack, communication_patterns, environments, code_conventions, testing_requirements, api_conventions, infrastructure, created_at, updated_at FROM board_settings WHERE board_id = ?",
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
        "SELECT board_id, codebase_path, github_repo, context_markdown, document_links, variables, tech_stack, communication_patterns, environments, code_conventions, testing_requirements, api_conventions, infrastructure, created_at, updated_at FROM board_settings WHERE board_id = ?",
    )
    .bind(&board_id)
    .fetch_optional(pool)
    .await?;

    let (cb, gr, cm, dl, va, ts, cp, en, cc, tr, ac, inf) = match &existing {
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
        ),
    };

    let settings: BoardSettings = sqlx::query_as(
        "INSERT INTO board_settings (board_id, codebase_path, github_repo, context_markdown, document_links, variables, tech_stack, communication_patterns, environments, code_conventions, testing_requirements, api_conventions, infrastructure, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
             updated_at = excluded.updated_at
         RETURNING board_id, codebase_path, github_repo, context_markdown, document_links, variables, tech_stack, communication_patterns, environments, code_conventions, testing_requirements, api_conventions, infrastructure, created_at, updated_at",
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
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;

    Ok(Json(settings))
}

pub async fn auto_detect_board_settings(
    State(_state): State<AppState>,
    Path(_board_id): Path<String>,
    Json(req): Json<AutoDetectBoardSettingsRequest>,
) -> Result<(StatusCode, Json<AutoDetectBoardSettingsResponse>), KanbanError> {
    let _codebase_path = req.codebase_path;

    Ok((
        StatusCode::ACCEPTED,
        Json(AutoDetectBoardSettingsResponse {
            status: "queued".to_string(),
        }),
    ))
}
