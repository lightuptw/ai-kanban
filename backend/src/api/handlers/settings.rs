use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::state::AppState;
use crate::domain::KanbanError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SetSettingRequest {
    pub value: String,
}

pub async fn get_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<Setting>, KanbanError> {
    let db = state.require_db()?;

    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT key, value, updated_at FROM settings WHERE key = ?")
            .bind(&key)
            .fetch_optional(db)
            .await?;

    match row {
        Some((k, v, u)) => Ok(Json(Setting {
            key: k,
            value: v,
            updated_at: u,
        })),
        None => Err(KanbanError::NotFound(format!("Setting '{}' not found", key))),
    }
}

pub async fn set_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<SetSettingRequest>,
) -> Result<Json<Setting>, KanbanError> {
    let db = state.require_db()?;
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(&key)
    .bind(&req.value)
    .bind(&now)
    .execute(db)
    .await?;

    Ok(Json(Setting {
        key,
        value: req.value,
        updated_at: now,
    }))
}
