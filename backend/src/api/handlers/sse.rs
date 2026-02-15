use axum::{
    response::sse::{Event, KeepAlive, Sse},
    extract::State,
};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::api::AppState;
use crate::domain::AgentLog;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SseEvent {
    CardCreated { card_id: String, title: String },
    CardUpdated { card_id: String },
    CardMoved { card_id: String, from_stage: String, to_stage: String },
    CardDeleted { card_id: String },
    SubtaskToggled { card_id: String, subtask_id: String, completed: bool },
    AiStatusChanged { card_id: String, status: String, progress: serde_json::Value, stage: String, ai_session_id: Option<String> },
    AgentLogCreated { card_id: String, log: AgentLog },
    QuestionCreated { card_id: String, question: serde_json::Value },
    QuestionAnswered { card_id: String, question: serde_json::Value },
}

pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_tx.subscribe();
    let stream = BroadcastStream::new(rx);

    let event_stream = stream
        .filter_map(|result| match result {
            Ok(event_json) => Some(Ok(Event::default().data(event_json))),
            Err(_) => None, // Skip lagged messages
        });

    Sse::new(event_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
