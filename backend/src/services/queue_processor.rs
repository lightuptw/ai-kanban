use std::time::Duration;

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
