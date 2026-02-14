use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;

use crate::api::state::AppState;
use crate::domain::KanbanError;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct CardFile {
    pub id: String,
    pub card_id: String,
    pub filename: String,
    pub original_filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub uploaded_at: String,
}

pub async fn upload_files(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Vec<CardFile>>), KanbanError> {
    let db = state.require_db()?;
    let mut uploaded_files = Vec::new();

    let card = sqlx::query_scalar::<_, String>("SELECT board_id FROM cards WHERE id = ?")
        .bind(&card_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| KanbanError::NotFound(format!("Card {} not found", card_id)))?;

    let upload_dir = PathBuf::from("uploads")
        .join(&card)
        .join(&card_id);
    
    fs::create_dir_all(&upload_dir).await.map_err(|e| {
        KanbanError::Internal(format!("Failed to create upload directory: {}", e))
    })?;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        KanbanError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let filename = field
            .file_name()
            .map(|s: &str| s.to_string())
            .unwrap_or_else(|| format!("file_{}", Uuid::new_v4()));

        let content_type = field
            .content_type()
            .map(|s: &str| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let data: Vec<u8> = field.bytes().await.map_err(|e| {
            KanbanError::BadRequest(format!("Failed to read file data: {}", e))
        })?.to_vec();

        let file_id = Uuid::new_v4().to_string();
        let sanitized_filename = sanitize_filename(&filename);
        let stored_filename = format!("{}_{}", file_id, sanitized_filename);
        let file_path = upload_dir.join(&stored_filename);

        let mut file = fs::File::create(&file_path).await.map_err(|e| {
            KanbanError::Internal(format!("Failed to create file: {}", e))
        })?;

        file.write_all(&data).await.map_err(|e| {
            KanbanError::Internal(format!("Failed to write file: {}", e))
        })?;

        let file_size = data.len() as i64;
        let now = chrono::Utc::now().to_rfc3339();
        let file_path_str = file_path.to_string_lossy().to_string();

        let card_file = sqlx::query_as::<_, CardFile>(
            r#"
            INSERT INTO card_files (id, card_id, filename, original_filename, file_path, file_size, mime_type, uploaded_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, card_id, filename, original_filename, file_size, mime_type, uploaded_at
            "#
        )
        .bind(&file_id)
        .bind(&card_id)
        .bind(&stored_filename)
        .bind(&filename)
        .bind(&file_path_str)
        .bind(file_size as i64)
        .bind(&content_type)
        .bind(&now)
        .fetch_one(db)
        .await?;

        uploaded_files.push(card_file);
    }

    Ok((StatusCode::CREATED, Json(uploaded_files)))
}

pub async fn list_card_files(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
) -> Result<Json<Vec<CardFile>>, KanbanError> {
    let db = state.require_db()?;

    let files = sqlx::query_as::<_, CardFile>(
        r#"
        SELECT id, card_id, filename, original_filename, file_size, mime_type, uploaded_at
        FROM card_files
        WHERE card_id = ?
        ORDER BY uploaded_at DESC
        "#
    )
    .bind(&card_id)
    .fetch_all(db)
    .await?;

    Ok(Json(files))
}

pub async fn download_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<impl IntoResponse, KanbanError> {
    let db = state.require_db()?;

    let row: (String, String, String) = sqlx::query_as(
        "SELECT file_path, original_filename, mime_type FROM card_files WHERE id = ?"
    )
    .bind(&file_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| KanbanError::NotFound(format!("File {} not found", file_id)))?;

    let (file_path, original_filename, mime_type) = row;

    let file_data = fs::read(&file_path).await.map_err(|e| {
        KanbanError::Internal(format!("Failed to read file: {}", e))
    })?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime_type),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", original_filename),
            ),
        ],
        file_data,
    ))
}

pub async fn delete_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<StatusCode, KanbanError> {
    let db = state.require_db()?;

    let file_path: String = sqlx::query_scalar("SELECT file_path FROM card_files WHERE id = ?")
        .bind(&file_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| KanbanError::NotFound(format!("File {} not found", file_id)))?;

    if let Err(e) = fs::remove_file(&file_path).await {
        tracing::warn!("Failed to delete file {}: {}", file_path, e);
    }

    sqlx::query("DELETE FROM card_files WHERE id = ?")
        .bind(&file_id)
        .execute(db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
