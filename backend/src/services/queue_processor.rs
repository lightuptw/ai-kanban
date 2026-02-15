use std::time::Duration;

use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::api::handlers::sse::SseEvent;
use crate::domain::{Card, KanbanError};

use super::{AiDispatchService, CardService};

pub struct QueueProcessor {
    pub db: SqlitePool,
    pub http_client: reqwest::Client,
    pub opencode_url: String,
    pub sse_tx: broadcast::Sender<String>,
}

impl QueueProcessor {
    pub async fn start(self) {
        loop {
            if let Err(e) = self.process_queue().await {
                tracing::warn!("Queue processor error: {}", e);
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }

    async fn process_queue(&self) -> Result<(), KanbanError> {
        self.recover_stuck_cards().await?;

        let concurrency_limit = self.get_concurrency_limit().await;
        let active_count = self.count_active_cards().await?;

        if active_count >= concurrency_limit {
            return Ok(());
        }

        let slots = concurrency_limit - active_count;
        let queued_cards = self.get_queued_cards(slots as i64).await?;

        for card in queued_cards {
            let subtasks = CardService::get_subtasks(&self.db, &card.id).await?;
            let dispatcher =
                AiDispatchService::new(self.http_client.clone(), self.opencode_url.clone());

            match dispatcher.dispatch_card(&card, &subtasks, &self.db).await {
                Ok(_) => {
                    let event = SseEvent::AiStatusChanged {
                        card_id: card.id.clone(),
                        status: "dispatched".to_string(),
                        progress: json!({}),
                        stage: card.stage.clone(),
                        ai_session_id: card.ai_session_id.clone(),
                    };
                    if let Ok(payload) = serde_json::to_string(&event) {
                        let _ = self.sse_tx.send(payload);
                    }
                }
                Err(e) => {
                    tracing::warn!(card_id = card.id, "Queue dispatch failed: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn recover_stuck_cards(&self) -> Result<(), KanbanError> {
        let timeout_minutes = sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = 'ai_stuck_timeout_minutes'",
        )
        .fetch_optional(&self.db)
        .await
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(10)
        .max(1);

        let cutoff = Utc::now() - chrono::Duration::minutes(timeout_minutes);
        let cards = sqlx::query_as::<_, Card>(
            "SELECT * FROM cards WHERE ai_status IN ('dispatched', 'working')",
        )
        .fetch_all(&self.db)
        .await?;

        for card in cards {
            let updated_at = match chrono::DateTime::parse_from_rfc3339(&card.updated_at) {
                Ok(parsed) => parsed.with_timezone(&Utc),
                Err(error) => {
                    tracing::warn!(
                        card_id = card.id,
                        updated_at = card.updated_at,
                        error = %error,
                        "Failed to parse card updated_at while checking stuck cards"
                    );
                    continue;
                }
            };

            if updated_at >= cutoff {
                continue;
            }

            let Some(session_id) = card.ai_session_id.as_deref() else {
                if let Err(error) = self.mark_card_failed_and_emit(&card.id).await {
                    tracing::warn!(
                        card_id = card.id,
                        error = %error,
                        "Failed to recover stuck card without session id"
                    );
                    continue;
                }

                tracing::warn!(
                    card_id = card.id,
                    ai_status = card.ai_status,
                    "Recovered stuck card with missing session id"
                );
                continue;
            };

            let session_url = format!("{}/session/{}", self.opencode_url, session_id);
            let is_stuck = match self.http_client.get(&session_url).send().await {
                Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => true,
                Ok(response) if !response.status().is_success() => false,
                Ok(response) => match response.json::<serde_json::Value>().await {
                    Ok(body) => {
                        let status = body
                            .get("status")
                            .and_then(|status| {
                                status
                                    .get("type")
                                    .and_then(serde_json::Value::as_str)
                                    .or_else(|| status.as_str())
                            })
                            .unwrap_or("");
                        status == "idle"
                    }
                    Err(error) => {
                        tracing::warn!(
                            card_id = card.id,
                            session_id,
                            error = %error,
                            "Failed to decode session status while checking stuck cards"
                        );
                        true
                    }
                },
                Err(error) => {
                    tracing::warn!(
                        card_id = card.id,
                        session_id,
                        error = %error,
                        "Failed to fetch session while checking stuck cards"
                    );
                    true
                }
            };

            if !is_stuck {
                continue;
            }

            if let Err(error) = self.mark_card_failed_and_emit(&card.id).await {
                tracing::warn!(
                    card_id = card.id,
                    session_id,
                    error = %error,
                    "Failed to recover stuck card"
                );
                continue;
            }

            tracing::warn!(
                card_id = card.id,
                session_id,
                ai_status = card.ai_status,
                "Recovered stuck card"
            );
        }

        Ok(())
    }

    async fn mark_card_failed_and_emit(&self, card_id: &str) -> Result<(), KanbanError> {
        sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
            .bind("failed")
            .bind(Utc::now().to_rfc3339())
            .bind(card_id)
            .execute(&self.db)
            .await?;

        let card: Option<Card> = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(card_id)
            .fetch_optional(&self.db)
            .await?;

        let event = SseEvent::AiStatusChanged {
            card_id: card_id.to_string(),
            status: "failed".to_string(),
            progress: json!({}),
            stage: card.as_ref().map(|c| c.stage.clone()).unwrap_or_default(),
            ai_session_id: card.and_then(|c| c.ai_session_id),
        };
        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = self.sse_tx.send(payload);
        }

        Ok(())
    }

    async fn get_concurrency_limit(&self) -> usize {
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = 'ai_concurrency'")
            .fetch_optional(&self.db)
            .await
            .ok()
            .flatten()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1)
            .max(1)
    }

    async fn count_active_cards(&self) -> Result<usize, KanbanError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM cards WHERE stage IN ('todo', 'in_progress') AND ai_status IN ('dispatched', 'working')",
        )
        .fetch_one(&self.db)
        .await?;

        Ok(count.0 as usize)
    }

    async fn get_queued_cards(&self, limit: i64) -> Result<Vec<Card>, KanbanError> {
        let cards = sqlx::query_as::<_, Card>(
            "SELECT * FROM cards WHERE stage = 'todo' AND ai_status = 'queued' ORDER BY updated_at ASC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(cards)
    }
}
