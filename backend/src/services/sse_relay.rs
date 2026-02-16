use std::time::Duration;

use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::api::handlers::sse::WsEvent;
use crate::domain::{AgentLog, Card};

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

                    if event_type != "server.heartbeat" {
                        tracing::info!(
                            event_type = event_type.as_str(),
                            "Received OpenCode SSE event"
                        );
                        tracing::debug!(
                            event_type = event_type.as_str(),
                            payload = payload.to_string(),
                            "OpenCode SSE event payload"
                        );
                    }

                    if let Err(err) = self.handle_opencode_event(&event_type, &payload).await {
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

    /// Extract session_id from opencode event properties.
    /// opencode events nest session ID in different locations:
    /// - `properties.sessionID` (session.status, session.idle, session.diff)
    /// - `properties.info.sessionID` (message.updated, session.updated)
    /// - `properties.part.sessionID` (message.part.updated, message.part.delta)
    fn extract_session_id(properties: &Value) -> Option<&str> {
        properties
            .get("sessionID")
            .and_then(Value::as_str)
            .or_else(|| {
                properties
                    .get("info")
                    .and_then(|info| info.get("sessionID"))
                    .and_then(Value::as_str)
            })
            .or_else(|| {
                properties
                    .get("part")
                    .and_then(|part| part.get("sessionID"))
                    .and_then(Value::as_str)
            })
    }

    async fn handle_opencode_event(&self, event_type: &str, properties: &Value) -> Result<()> {
        let Some(session_id) = Self::extract_session_id(properties) else {
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

        let should_log = match event_type {
            "message.part.updated" | "session.diff" | "server.connected" | "server.heartbeat" => {
                false
            }
            "message.updated" => properties
                .get("info")
                .and_then(|info| info.get("finish"))
                .and_then(Value::as_str)
                .is_some(),
            _ => true,
        };

        if should_log {
            let log = self
                .create_agent_log(&card, session_id, event_type, properties)
                .await?;
            let log_event = WsEvent::AgentLogCreated {
                card_id: card.id.clone(),
                log,
            };
            if let Ok(payload) = serde_json::to_string(&log_event) {
                let _ = self.sse_tx.send(payload);
            }
        }

        let now = Utc::now().to_rfc3339();

        match event_type {
            "session.status" => {
                let status_type = properties
                    .get("status")
                    .and_then(|s| s.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("");

                match status_type {
                    "busy" => {
                        if card.stage == "todo" {
                            tracing::info!(
                                card_id = card.id,
                                session_id,
                                "AI session busy → moving card to in_progress"
                            );
                            sqlx::query(
                                "UPDATE cards SET ai_status = ?, stage = ?, updated_at = ? WHERE id = ?",
                            )
                            .bind("working")
                            .bind("in_progress")
                            .bind(&now)
                            .bind(&card.id)
                            .execute(&self.db)
                            .await?;
                        } else if card.stage == "plan" && card.ai_status == "planning" {
                            sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                                .bind("working")
                                .bind(&now)
                                .bind(&card.id)
                                .execute(&self.db)
                                .await?;
                        }
                    }
                    _ => {
                        return Ok(());
                    }
                }
            }

            "session.idle" => {
                if card.stage == "in_progress" {
                    tracing::info!(
                        card_id = card.id,
                        session_id,
                        "AI session idle → moving card to review"
                    );
                    sqlx::query(
                        "UPDATE cards SET ai_status = ?, stage = ?, updated_at = ? WHERE id = ?",
                    )
                    .bind("completed")
                    .bind("review")
                    .bind(&now)
                    .bind(&card.id)
                    .execute(&self.db)
                    .await?;
                } else if card.stage == "plan" {
                    sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
                        .bind("idle")
                        .bind(&now)
                        .bind(&card.id)
                        .execute(&self.db)
                        .await?;
                }
            }

            "message.updated" => {
                let agent = properties
                    .get("info")
                    .and_then(|info| info.get("agent"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");

                let finish = properties
                    .get("info")
                    .and_then(|info| info.get("finish"))
                    .and_then(Value::as_str);

                let mut progress: Value =
                    serde_json::from_str(&card.ai_progress).unwrap_or_else(|_| json!({}));
                progress["current_agent"] = json!(agent);

                if let Some(finish_reason) = finish {
                    progress["last_finish_reason"] = json!(finish_reason);
                }

                sqlx::query("UPDATE cards SET ai_progress = ?, updated_at = ? WHERE id = ?")
                    .bind(progress.to_string())
                    .bind(&now)
                    .bind(&card.id)
                    .execute(&self.db)
                    .await?;
            }

            "todo.updated" => {
                let mut progress: Value =
                    serde_json::from_str(&card.ai_progress).unwrap_or_else(|_| json!({}));

                if let Some(todos) = properties.get("todos").and_then(Value::as_array) {
                    let total = todos.len();
                    let completed = todos
                        .iter()
                        .filter(|t| {
                            t.get("status")
                                .or_else(|| t.get("state"))
                                .and_then(Value::as_str)
                                == Some("completed")
                        })
                        .count();
                    progress["total_todos"] = json!(total);
                    progress["completed_todos"] = json!(completed);

                    if let Some(current) = todos.iter().find(|t| {
                        t.get("status")
                            .or_else(|| t.get("state"))
                            .and_then(Value::as_str)
                            == Some("in_progress")
                    }) {
                        if let Some(content) = current
                            .get("content")
                            .or_else(|| current.get("text"))
                            .and_then(Value::as_str)
                        {
                            progress["current_task"] = json!(content);
                        }
                    }
                }

                sqlx::query("UPDATE cards SET ai_progress = ?, updated_at = ? WHERE id = ?")
                    .bind(progress.to_string())
                    .bind(&now)
                    .bind(&card.id)
                    .execute(&self.db)
                    .await?;
            }

            _ => {
                return Ok(());
            }
        }

        let updated_card = CardService::get_card_model(&self.db, &card.id).await?;
        let progress =
            serde_json::from_str(&updated_card.ai_progress).unwrap_or_else(|_| json!({}));
        let event = WsEvent::AiStatusChanged {
            card_id: updated_card.id.clone(),
            status: updated_card.ai_status.clone(),
            progress: progress.clone(),
            stage: updated_card.stage.clone(),
            ai_session_id: updated_card.ai_session_id.clone(),
        };

        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = self.sse_tx.send(payload);
        }

        if updated_card.stage != card.stage {
            let move_event = WsEvent::CardMoved {
                card_id: updated_card.id,
                from_stage: card.stage.clone(),
                to_stage: updated_card.stage,
            };
            if let Ok(payload) = serde_json::to_string(&move_event) {
                let _ = self.sse_tx.send(payload);
            }
        }

        Ok(())
    }

    /// Extract event type and properties from opencode SSE data.
    ///
    /// opencode SSE format:
    /// - SSE `event:` field is NOT set (defaults to "message")
    /// - SSE `data:` field contains JSON: `{"type": "session.status", "properties": {...}}`
    fn extract_event_type_and_payload(_raw_event: &str, raw_data: &str) -> Option<(String, Value)> {
        let parsed_data: Value = serde_json::from_str(raw_data).ok()?;

        let event_type = parsed_data
            .get("type")
            .and_then(Value::as_str)?
            .to_string();

        let properties = parsed_data
            .get("properties")
            .cloned()
            .unwrap_or(json!({}));

        Some((event_type, properties))
    }

    async fn create_agent_log(
        &self,
        card: &Card,
        session_id: &str,
        event_type: &str,
        properties: &Value,
    ) -> Result<AgentLog> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let agent = properties
            .get("info")
            .and_then(|info| info.get("agent"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let content = Self::build_log_content(event_type, properties, agent.as_deref());
        let metadata = properties.to_string();

        sqlx::query(
            "INSERT INTO agent_logs (id, card_id, session_id, event_type, agent, content, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&card.id)
        .bind(session_id)
        .bind(event_type)
        .bind(agent.as_deref())
        .bind(&content)
        .bind(&metadata)
        .bind(&created_at)
        .execute(&self.db)
        .await?;

        Ok(AgentLog {
            id,
            card_id: card.id.clone(),
            session_id: session_id.to_string(),
            event_type: event_type.to_string(),
            agent,
            content,
            metadata,
            created_at,
        })
    }

    fn build_log_content(event_type: &str, properties: &Value, agent: Option<&str>) -> String {
        match event_type {
            "message.part.delta" => properties
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            "message.updated" => {
                let finish = properties
                    .get("info")
                    .and_then(|info| info.get("finish"))
                    .and_then(Value::as_str);
                if let Some(finish_reason) = finish {
                    format!(
                        "Agent {} finished ({})",
                        agent.unwrap_or("unknown"),
                        finish_reason
                    )
                } else {
                    format!("Agent {} updated message", agent.unwrap_or("unknown"))
                }
            }
            "session.status" => {
                let status_type = properties
                    .get("status")
                    .and_then(|s| s.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                format!("Session {}", status_type)
            }
            "session.idle" => "Session completed".to_string(),
            "todo.updated" => Self::summarize_todos(properties),
            _ => format!("Event {}", event_type),
        }
    }

    fn summarize_todos(properties: &Value) -> String {
        let Some(todos) = properties.get("todos").and_then(Value::as_array) else {
            return "Todo list updated".to_string();
        };

        let total = todos.len();
        let completed = todos
            .iter()
            .filter(|t| {
                t.get("status")
                    .or_else(|| t.get("state"))
                    .and_then(Value::as_str)
                    == Some("completed")
            })
            .count();
        let in_progress = todos
            .iter()
            .find(|t| {
                t.get("status")
                    .or_else(|| t.get("state"))
                    .and_then(Value::as_str)
                    == Some("in_progress")
            })
            .and_then(|t| t.get("content").or_else(|| t.get("text")))
            .and_then(Value::as_str);

        match in_progress {
            Some(task) => format!(
                "Todos updated: {}/{} completed, in progress: {}",
                completed, total, task
            ),
            None => format!("Todos updated: {}/{} completed", completed, total),
        }
    }
}
