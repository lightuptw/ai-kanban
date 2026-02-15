use serde::{Deserialize, Serialize};

use crate::domain::{Card, Comment, Label, Subtask};

#[derive(Debug, Deserialize)]
pub struct CreateCardRequest {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub board_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCardRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub linked_documents: Option<String>,
    #[serde(default)]
    pub ai_agent: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveCardRequest {
    pub stage: String,
    #[serde(default)]
    pub position: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CardResponse {
    pub id: String,
    pub title: String,
    pub description: String,
    pub stage: String,
    pub position: i64,
    pub priority: String,
    pub working_directory: String,
    pub plan_path: Option<String>,
    pub ai_session_id: Option<String>,
    pub ai_status: String,
    pub ai_progress: serde_json::Value,
    pub linked_documents: String,
    pub created_at: String,
    pub updated_at: String,
    pub ai_agent: Option<String>,
    pub subtasks: Vec<Subtask>,
    pub labels: Vec<Label>,
    pub comments: Vec<Comment>,
}

impl CardResponse {
    pub fn from_card(
        card: Card,
        subtasks: Vec<Subtask>,
        labels: Vec<Label>,
        comments: Vec<Comment>,
    ) -> Self {
        let ai_progress = serde_json::from_str(&card.ai_progress).unwrap_or(serde_json::json!({}));
        Self {
            id: card.id,
            title: card.title,
            description: card.description,
            stage: card.stage,
            position: card.position,
            priority: card.priority,
            working_directory: card.working_directory,
            plan_path: card.plan_path,
            ai_session_id: card.ai_session_id,
            ai_status: card.ai_status,
            ai_progress,
            linked_documents: card.linked_documents,
            created_at: card.created_at,
            updated_at: card.updated_at,
            ai_agent: card.ai_agent,
            subtasks,
            labels,
            comments,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CardSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub stage: String,
    pub position: i64,
    pub priority: String,
    pub ai_status: String,
    pub ai_agent: Option<String>,
    pub subtask_count: i64,
    pub subtask_completed: i64,
    pub label_count: i64,
    pub comment_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct BoardResponse {
    pub backlog: Vec<CardSummary>,
    pub plan: Vec<CardSummary>,
    pub todo: Vec<CardSummary>,
    pub in_progress: Vec<CardSummary>,
    pub review: Vec<CardSummary>,
    pub done: Vec<CardSummary>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubtaskRequest {
    pub title: String,
    #[serde(default = "default_phase_name")]
    pub phase: String,
    #[serde(default = "default_phase_order")]
    pub phase_order: i64,
}

fn default_phase_name() -> String {
    "Phase 1".into()
}

fn default_phase_order() -> i64 {
    1
}

#[derive(Debug, Deserialize)]
pub struct UpdateSubtaskRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub completed: Option<bool>,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub phase_order: Option<i64>,
    #[serde(default)]
    pub position: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    #[serde(default)]
    pub author: Option<String>,
}
