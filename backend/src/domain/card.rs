use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;

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
    pub branch_name: String,
    pub worktree_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    CardStageChanged,
    AiCompleted,
    AiQuestionPending,
    ReviewRequested,
    AiError,
}

impl fmt::Display for NotificationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CardStageChanged => write!(f, "card_stage_changed"),
            Self::AiCompleted => write!(f, "ai_completed"),
            Self::AiQuestionPending => write!(f, "ai_question_pending"),
            Self::ReviewRequested => write!(f, "review_requested"),
            Self::AiError => write!(f, "ai_error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notification {
    pub id: String,
    pub user_id: Option<String>,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub card_id: Option<String>,
    pub board_id: Option<String>,
    pub is_read: bool,
    pub created_at: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiQuestion {
    pub id: String,
    pub card_id: String,
    pub session_id: String,
    pub question: String,
    pub question_type: String,
    pub options: String,
    pub multiple: bool,
    pub answer: Option<String>,
    pub answered_at: Option<String>,
    pub created_at: String,
}
