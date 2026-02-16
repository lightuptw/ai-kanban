use std::time::Duration;

use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::api::handlers::sse::WsEvent;
use crate::domain::{Card, KanbanError};

use super::{AiDispatchService, CardService, GitWorktreeService};

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

        let queued_board_ids = self.get_queued_board_ids().await?;

        for board_id in queued_board_ids {
            let concurrency_limit = self.get_board_concurrency_limit(&board_id).await;
            let active_count = self.count_active_cards(&board_id).await?;

            if active_count >= concurrency_limit {
                continue;
            }

            let slots = concurrency_limit.saturating_sub(active_count);
            let queued_cards = self
                .get_queued_cards(&board_id, std::cmp::min(slots, i64::MAX as usize) as i64)
                .await?;

            for card in queued_cards {
                let mut dispatch_card = card;

                if dispatch_card.worktree_path.is_empty() {
                    let codebase_path: Option<String> = sqlx::query_scalar(
                        "SELECT codebase_path FROM board_settings WHERE board_id = ?",
                    )
                    .bind(&board_id)
                    .fetch_optional(&self.db)
                    .await
                    .ok()
                    .flatten();

                    if let Some(codebase) = codebase_path {
                        if !codebase.is_empty() {
                            match GitWorktreeService::create_worktree(
                                &codebase,
                                &dispatch_card.id,
                                &dispatch_card.title,
                            ) {
                                Ok((branch_name, worktree_path)) => {
                                    if let Err(error) = sqlx::query(
                                        "UPDATE cards SET branch_name = ?, worktree_path = ?, working_directory = ?, updated_at = ? WHERE id = ?",
                                    )
                                    .bind(&branch_name)
                                    .bind(&worktree_path)
                                    .bind(&worktree_path)
                                    .bind(chrono::Utc::now().to_rfc3339())
                                    .bind(&dispatch_card.id)
                                    .execute(&self.db)
                                    .await
                                    {
                                        tracing::warn!(
                                            card_id = dispatch_card.id,
                                            error = %error,
                                            "Failed to persist worktree paths; dispatching with original working directory"
                                        );
                                    } else {
                                        match sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = ?")
                                            .bind(&dispatch_card.id)
                                            .fetch_one(&self.db)
                                            .await
                                        {
                                            Ok(updated_card) => {
                                                dispatch_card = updated_card;
                                            }
                                            Err(error) => {
                                                tracing::warn!(
                                                    card_id = dispatch_card.id,
                                                    error = %error,
                                                    "Failed to reload card after worktree creation"
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(error) => {
                                    tracing::warn!(
                                        card_id = dispatch_card.id,
                                        error = %error,
                                        "Failed to create worktree; dispatching with original working directory"
                                    );
                                }
                            }
                        }
                    }
                }

                let subtasks = CardService::get_subtasks(&self.db, &dispatch_card.id).await?;
                let dispatcher =
                    AiDispatchService::new(self.http_client.clone(), self.opencode_url.clone());

                match dispatcher.dispatch_card(&dispatch_card, &subtasks, &self.db).await {
                    Ok(session_id) if !session_id.is_empty() => {
                        if let Err(e) = sqlx::query(
                            "UPDATE cards SET stage = ?, updated_at = ? WHERE id = ?",
                        )
                        .bind("in_progress")
                        .bind(chrono::Utc::now().to_rfc3339())
                        .bind(&dispatch_card.id)
                        .execute(&self.db)
                        .await
                        {
                            tracing::warn!(
                                card_id = dispatch_card.id,
                                error = %e,
                                "Failed to move card to in_progress after dispatch"
                            );
                        }

                        let move_event = WsEvent::CardMoved {
                            card_id: dispatch_card.id.clone(),
                            from_stage: "todo".to_string(),
                            to_stage: "in_progress".to_string(),
                        };
                        if let Ok(payload) = serde_json::to_string(&move_event) {
                            let _ = self.sse_tx.send(payload);
                        }

                        let event = WsEvent::AiStatusChanged {
                            card_id: dispatch_card.id.clone(),
                            status: "dispatched".to_string(),
                            progress: json!({}),
                            stage: "in_progress".to_string(),
                            ai_session_id: Some(session_id),
                        };
                        if let Ok(payload) = serde_json::to_string(&event) {
                            let _ = self.sse_tx.send(payload);
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(card_id = dispatch_card.id, "Queue dispatch failed: {}", e);
                    }
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

        let event = WsEvent::AiStatusChanged {
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

    async fn get_board_concurrency_limit(&self, board_id: &str) -> usize {
        let concurrency = sqlx::query_scalar::<_, i64>(
            "SELECT ai_concurrency FROM board_settings WHERE board_id = ?",
        )
        .bind(board_id)
            .fetch_optional(&self.db)
            .await
            .ok()
            .flatten()
            .unwrap_or(1);

        if concurrency == 0 {
            usize::MAX
        } else {
            concurrency.max(1) as usize
        }
    }

    async fn get_queued_board_ids(&self) -> Result<Vec<String>, KanbanError> {
        let board_ids = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT board_id FROM cards WHERE stage = 'todo' AND ai_status = 'queued'",
        )
        .fetch_all(&self.db)
        .await?;

        Ok(board_ids)
    }

    async fn count_active_cards(&self, board_id: &str) -> Result<usize, KanbanError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM cards WHERE board_id = ? AND stage IN ('todo', 'in_progress') AND ai_status IN ('dispatched', 'working')",
        )
        .bind(board_id)
        .fetch_one(&self.db)
        .await?;

        Ok(count.0 as usize)
    }

    async fn get_queued_cards(&self, board_id: &str, limit: i64) -> Result<Vec<Card>, KanbanError> {
        let cards = sqlx::query_as::<_, Card>(
            "SELECT * FROM cards WHERE board_id = ? AND stage = 'todo' AND ai_status = 'queued' ORDER BY CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 WHEN 'low' THEN 2 ELSE 3 END ASC, updated_at ASC LIMIT ?",
        )
        .bind(board_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(cards)
    }
}
