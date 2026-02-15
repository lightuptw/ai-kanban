use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::api::AppState;
use crate::domain::{AiQuestion, KanbanError};

#[derive(Debug, Deserialize)]
pub struct CreateQuestionRequest {
    pub question: String,
    #[serde(default = "default_select")]
    pub question_type: String,
    #[serde(default = "default_empty_array")]
    pub options: String,
    #[serde(default)]
    pub multiple: bool,
}

#[derive(Debug, Deserialize)]
pub struct AnswerQuestionRequest {
    pub answer: Value,
}

fn default_select() -> String {
    "select".to_string()
}

fn default_empty_array() -> String {
    "[]".to_string()
}

pub async fn get_questions(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
) -> Result<Json<Vec<AiQuestion>>, KanbanError> {
    let pool = state.require_db()?;
    let questions: Vec<AiQuestion> = sqlx::query_as(
        "SELECT * FROM ai_questions WHERE card_id = ? ORDER BY created_at ASC",
    )
    .bind(&card_id)
    .fetch_all(pool)
    .await?;
    Ok(Json(questions))
}

pub async fn create_question(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
    Json(req): Json<CreateQuestionRequest>,
) -> Result<(StatusCode, Json<AiQuestion>), KanbanError> {
    let pool = state.require_db()?;

    let question_type = req.question_type.trim().to_lowercase();
    if !matches!(question_type.as_str(), "select" | "multi_select" | "text") {
        return Err(KanbanError::BadRequest(
            "question_type must be one of: select, multi_select, text".into(),
        ));
    }

    let options_value: Value = serde_json::from_str(&req.options)
        .map_err(|e| KanbanError::BadRequest(format!("options must be valid JSON: {}", e)))?;
    if !options_value.is_array() {
        return Err(KanbanError::BadRequest(
            "options must be a JSON array".into(),
        ));
    }

    let session_id: String = sqlx::query_scalar("SELECT ai_session_id FROM cards WHERE id = ?")
        .bind(&card_id)
        .fetch_optional(pool)
        .await?
        .flatten()
        .ok_or_else(|| {
            KanbanError::BadRequest("Card has no active AI session for asking questions".into())
        })?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO ai_questions (id, card_id, session_id, question, question_type, options, multiple, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&card_id)
    .bind(&session_id)
    .bind(&req.question)
    .bind(&question_type)
    .bind(options_value.to_string())
    .bind(req.multiple)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
        .bind("waiting_input")
        .bind(&now)
        .bind(&card_id)
        .execute(pool)
        .await?;

    let question_row: AiQuestion =
        sqlx::query_as("SELECT * FROM ai_questions WHERE id = ?")
            .bind(&id)
            .fetch_one(pool)
            .await?;

    let event = json!({
        "type": "QuestionCreated",
        "card_id": card_id,
        "question": question_row,
    });
    let _ = state
        .sse_tx
        .send(serde_json::to_string(&event).unwrap_or_default());

    Ok((StatusCode::CREATED, Json(question_row)))
}

pub async fn answer_question(
    State(state): State<AppState>,
    Path((card_id, question_id)): Path<(String, String)>,
    Json(req): Json<AnswerQuestionRequest>,
) -> Result<Json<AiQuestion>, KanbanError> {
    let pool = state.require_db()?;

    let question: AiQuestion =
        sqlx::query_as("SELECT * FROM ai_questions WHERE id = ? AND card_id = ?")
            .bind(&question_id)
            .bind(&card_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                KanbanError::NotFound(format!(
                    "Question not found: {} for card {}",
                    question_id, card_id
                ))
            })?;

    let answer = match question.question_type.as_str() {
        "text" => match req.answer {
            Value::String(text) => text,
            _ => {
                return Err(KanbanError::BadRequest(
                    "answer must be a string for text questions".into(),
                ));
            }
        },
        "select" | "multi_select" => match req.answer {
            Value::Array(items) => serde_json::to_string(&items).map_err(|e| {
                KanbanError::Internal(format!("Failed to serialize answer array: {}", e))
            })?,
            _ => {
                return Err(KanbanError::BadRequest(
                    "answer must be an array for select and multi_select questions".into(),
                ));
            }
        },
        _ => {
            return Err(KanbanError::BadRequest(format!(
                "Unsupported question_type: {}",
                question.question_type
            )));
        }
    };

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE ai_questions SET answer = ?, answered_at = ? WHERE id = ? AND card_id = ?")
        .bind(&answer)
        .bind(&now)
        .bind(&question_id)
        .bind(&card_id)
        .execute(pool)
        .await?;

    sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
        .bind("working")
        .bind(&now)
        .bind(&card_id)
        .execute(pool)
        .await?;

    let question_row: AiQuestion =
        sqlx::query_as("SELECT * FROM ai_questions WHERE id = ?")
            .bind(&question_id)
            .fetch_one(pool)
            .await?;

    let event = json!({
        "type": "QuestionAnswered",
        "card_id": card_id,
        "question": question_row,
    });
    let _ = state
        .sse_tx
        .send(serde_json::to_string(&event).unwrap_or_default());

    Ok(Json(question_row))
}
