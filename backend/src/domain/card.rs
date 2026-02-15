use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Card {
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
    pub ai_progress: String,
    pub linked_documents: String,
    pub created_at: String,
    pub updated_at: String,
    pub ai_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subtask {
    pub id: String,
    pub card_id: String,
    pub title: String,
    pub completed: bool,
    pub position: i64,
    pub phase: String,
    pub phase_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Comment {
    pub id: String,
    pub card_id: String,
    pub author: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentLog {
    pub id: String,
    pub card_id: String,
    pub session_id: String,
    pub event_type: String,
    pub agent: Option<String>,
    pub content: String,
    pub metadata: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CardVersion {
    pub id: String,
    pub card_id: String,
    pub snapshot: String,
    pub changed_by: String,
    pub created_at: String,
}
