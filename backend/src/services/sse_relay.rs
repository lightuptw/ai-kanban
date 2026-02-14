use std::time::Duration;

use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::api::handlers::sse::SseEvent;
use crate::domain::Card;

use super::CardService;

pub struct SseRelayService {
    pub opencode_url: String,
    pub db: SqlitePool,
    pub sse_tx: broadcast::Sender<String>,
    pub http_client: reqwest::Client,
}

impl SseRelayService {
    pub async fn start(self) {
        let mut backoff_seconds = 1;

        loop {
            match self.connect_and_relay().await {
                Ok(()) => {
                    tracing::info!("SSE relay disconnected, reconnecting...");
                    backoff_seconds = 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "SSE relay error: {}, retrying in {}s",
                        e,
                        backoff_seconds
                    );
                    tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;
                    backoff_seconds = (backoff_seconds * 2).min(30);
                }
            }
        }
    }

    async fn connect_and_relay(&self) -> Result<()> {
        let endpoint = format!("{}/event", self.opencode_url.trim_end_matches('/'));
        tracing::info!(url = endpoint.as_str(), "Connecting OpenCode SSE relay");

        let request = self.http_client.get(endpoint);
        let mut event_source = EventSource::new(request)?;

        while let Some(next_event) = event_source.next().await {
            match next_event {
                Ok(Event::Open) => {
                    tracing::info!("OpenCode SSE relay connected");
                }
                Ok(Event::Message(message)) => {
                    let Some((event_type, payload)) =
                        Self::extract_event_type_and_payload(&message.event, &message.data)
                    else {
                        tracing::warn!(
                            raw_event = message.event,
                            raw_data = message.data,
                            "Skipping OpenCode SSE message with invalid payload"
                        );
                        continue;
                    };

                    tracing::info!(
                        event_type = event_type.as_str(),
                        payload = payload.to_string(),
                        "Received OpenCode SSE event"
                    );

                    if let Err(err) = self.handle_opencode_event(&event_type, payload).await {
                        tracing::error!(
                            error = %err,
                            event_type = event_type.as_str(),
                            "Failed to process OpenCode SSE event"
                        );
                    }
                }
                Err(err) => {
                    event_source.close();
                    return Err(anyhow!(err));
                }
            }
        }

        event_source.close();
        Ok(())
    }

    async fn handle_opencode_event(&self, event_type: &str, data: Value) -> Result<()> {
        let Some(session_id) = data
            .get("session_id")
            .and_then(Value::as_str)
            .or_else(|| data.get("sessionId").and_then(Value::as_str))
        else {
            tracing::warn!(event_type, "Skipping OpenCode event without session_id");
            return Ok(());
        };

        let card: Option<Card> = sqlx::query_as("SELECT * FROM cards WHERE ai_session_id = ?")
            .bind(session_id)
            .fetch_optional(&self.db)
            .await?;

        let Some(card) = card else {
            tracing::debug!(session_id, "Ignoring OpenCode event for unknown session");
            return Ok(());
        };

        let now = Utc::now().to_rfc3339();

        match event_type {
            "session_started" => {
                sqlx::query("UPDATE cards SET ai_status = ?, stage = ?, updated_at = ? WHERE id = ?")
                    .bind("working")
                    .bind("in_progress")
                    .bind(&now)
                    .bind(&card.id)
                    .execute(&self.db)
                    .await?;
            }
            "todo_completed" => {
                let mut progress: Value = serde_json::from_str(&card.ai_progress).unwrap_or_else(|_| json!({}));
                let completed = progress
                    .get("completed_todos")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    + 1;
                progress["completed_todos"] = json!(completed);

                if let Some(task) = data
                    .get("task_description")
                    .or_else(|| data.get("taskDescription"))
                    .or_else(|| data.get("current_task"))
                {
                    progress["current_task"] = task.clone();
                }

                if let Some(total) = data
                    .get("total_todos")
                    .or_else(|| data.get("totalTodos"))
                    .and_then(Value::as_i64)
                {
                    progress["total_todos"] = json!(total);
                }

                sqlx::query("UPDATE cards SET ai_progress = ?, updated_at = ? WHERE id = ?")
                    .bind(progress.to_string())
                    .bind(&now)
                    .bind(&card.id)
                    .execute(&self.db)
                    .await?;
            }
            "session_completed" => {
                sqlx::query("UPDATE cards SET ai_status = ?, stage = ?, updated_at = ? WHERE id = ?")
                    .bind("completed")
                    .bind("review")
                    .bind(&now)
                    .bind(&card.id)
                    .execute(&self.db)
                    .await?;
            }
            "session_error" => {
                sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                    .bind("failed")
                    .bind(&now)
                    .bind(&card.id)
                    .execute(&self.db)
                    .await?;

                let error_msg = data
                    .get("error")
                    .or_else(|| data.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown error");
                let comment_id = Uuid::new_v4().to_string();

                sqlx::query(
                    "INSERT INTO comments (id, card_id, author, content, created_at) VALUES (?, ?, 'ai', ?, ?)",
                )
                .bind(&comment_id)
                .bind(&card.id)
                .bind(format!("AI session failed: {}", error_msg))
                .bind(&now)
                .execute(&self.db)
                .await?;
            }
            _ => {
                tracing::debug!(event_type, "Ignoring unknown OpenCode event type");
                return Ok(());
            }
        }

        let updated_card = CardService::get_card_model(&self.db, &card.id).await?;
        let progress = serde_json::from_str(&updated_card.ai_progress).unwrap_or_else(|_| json!({}));
        let event = SseEvent::AiStatusChanged {
            card_id: updated_card.id,
            status: updated_card.ai_status,
            progress,
        };

        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = self.sse_tx.send(payload);
        }

        Ok(())
    }

    fn extract_event_type_and_payload(raw_event: &str, raw_data: &str) -> Option<(String, Value)> {
        let parsed_data: Value = serde_json::from_str(raw_data).ok()?;

        let event_type = if raw_event.is_empty() {
            parsed_data
                .get("type")
                .or_else(|| parsed_data.get("event"))
                .or_else(|| parsed_data.get("name"))
                .and_then(Value::as_str)
                .map(Self::normalize_event_type)
        } else {
            Some(Self::normalize_event_type(raw_event))
        }?;

        let payload = parsed_data
            .get("data")
            .filter(|inner| inner.is_object())
            .cloned()
            .unwrap_or(parsed_data);

        Some((event_type, payload))
    }

    fn normalize_event_type(raw: &str) -> String {
        match raw {
            "session.started" | "session-started" => "session_started".to_string(),
            "todo.completed" | "todo-completed" => "todo_completed".to_string(),
            "session.completed" | "session-completed" => "session_completed".to_string(),
            "session.error" | "session-error" => "session_error".to_string(),
            _ => raw.replace('.', "_").replace('-', "_"),
        }
    }
}
