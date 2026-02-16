use crate::domain::AgentLog;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsEvent {
    CardCreated {
        card: serde_json::Value,
    },
    CardUpdated {
        card: serde_json::Value,
    },
    CardMoved {
        card_id: String,
        from_stage: String,
        to_stage: String,
    },
    CardDeleted {
        card_id: String,
    },
    SubtaskCreated {
        card_id: String,
        subtask: serde_json::Value,
    },
    SubtaskUpdated {
        card_id: String,
        subtask: serde_json::Value,
    },
    SubtaskDeleted {
        card_id: String,
        subtask_id: String,
    },
    SubtaskToggled {
        card_id: String,
        subtask_id: String,
        completed: bool,
    },
    CommentCreated {
        card_id: String,
        comment: serde_json::Value,
    },
    CommentUpdated {
        card_id: String,
        comment: serde_json::Value,
    },
    CommentDeleted {
        card_id: String,
        comment_id: String,
    },
    BoardCreated {
        board: serde_json::Value,
    },
    BoardUpdated {
        board: serde_json::Value,
    },
    BoardDeleted {
        board_id: String,
    },
    LabelAdded {
        card_id: String,
        label_id: String,
    },
    LabelRemoved {
        card_id: String,
        label_id: String,
    },
    AiStatusChanged {
        card_id: String,
        status: String,
        progress: serde_json::Value,
        stage: String,
        ai_session_id: Option<String>,
    },
    AgentLogCreated {
        card_id: String,
        log: AgentLog,
    },
    QuestionCreated {
        card_id: String,
        question: serde_json::Value,
    },
    QuestionAnswered {
        card_id: String,
        question: serde_json::Value,
    },
    AutoDetectStatus {
        board_id: String,
        status: String,
        session_id: Option<String>,
        elapsed_seconds: Option<u64>,
        message: Option<String>,
    },
}
