use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::handlers::sse::WsEvent;
use crate::api::state::AppState;
use crate::domain::KanbanError;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateBoardRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBoardRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ReorderBoardRequest {
    pub position: i64,
}

pub async fn list_boards(State(state): State<AppState>) -> Result<Json<Vec<Board>>, KanbanError> {
    let db = state.require_db()?;

    let boards: Vec<Board> = sqlx::query_as(
        "SELECT id, name, position, created_at, updated_at FROM boards ORDER BY position ASC"
    )
    .fetch_all(db)
    .await?;

    Ok(Json(boards))
}

pub async fn create_board(
    State(state): State<AppState>,
    Json(req): Json<CreateBoardRequest>,
) -> Result<(StatusCode, Json<Board>), KanbanError> {
    let db = state.require_db()?;
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let max_pos: Option<(i64,)> = sqlx::query_as("SELECT COALESCE(MAX(position), 0) FROM boards")
        .fetch_optional(db)
        .await?;
    let position = max_pos.map(|r| r.0).unwrap_or(0) + 1000;

    let board: Board = sqlx::query_as(
        "INSERT INTO boards (id, name, position, created_at, updated_at) VALUES (?, ?, ?, ?, ?) RETURNING id, name, position, created_at, updated_at"
    )
    .bind(&id)
    .bind(&req.name)
    .bind(position)
    .bind(&now)
    .bind(&now)
    .fetch_one(db)
    .await?;

    let event = WsEvent::BoardCreated {
        board: serde_json::to_value(&board).unwrap_or_default(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok((StatusCode::CREATED, Json(board)))
}

pub async fn update_board(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateBoardRequest>,
) -> Result<Json<Board>, KanbanError> {
    let db = state.require_db()?;
    let now = chrono::Utc::now().to_rfc3339();

    let board: Board = sqlx::query_as(
        "UPDATE boards SET name = ?, updated_at = ? WHERE id = ? RETURNING id, name, position, created_at, updated_at"
    )
    .bind(&req.name)
    .bind(&now)
    .bind(&id)
    .fetch_one(db)
    .await?;

    let event = WsEvent::BoardUpdated {
        board: serde_json::to_value(&board).unwrap_or_default(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok(Json(board))
}

pub async fn delete_board(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, KanbanError> {
    let db = state.require_db()?;

    let result = sqlx::query("DELETE FROM boards WHERE id = ?")
        .bind(&id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(KanbanError::NotFound(format!("Board {} not found", id)));
    }

    let event = WsEvent::BoardDeleted {
        board_id: id.clone(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn reorder_board(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ReorderBoardRequest>,
) -> Result<Json<Board>, KanbanError> {
    let db = state.require_db()?;
    let now = chrono::Utc::now().to_rfc3339();

    let board: Board = sqlx::query_as(
        "UPDATE boards SET position = ?, updated_at = ? WHERE id = ? RETURNING id, name, position, created_at, updated_at"
    )
    .bind(req.position)
    .bind(&now)
    .bind(&id)
    .fetch_one(db)
    .await?;

    let event = WsEvent::BoardUpdated {
        board: serde_json::to_value(&board).unwrap_or_default(),
    };
    if let Ok(payload) = serde_json::to_string(&event) {
        let _ = state.sse_tx.send(payload);
    }

    Ok(Json(board))
}
