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
                            board_id: dispatch_card.board_id.clone(),
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
            "SELECT * FROM cards WHERE ai_status IN ('dispatched', 'working', 'waiting')",
        )
        .fetch_all(&self.db)
        .await?;

        for card in cards {
            if card.ai_status == "waiting" {
                self.check_waiting_card(&card).await;
                continue;
            }

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
                let target_status = if card.stage == "review" {
                    "completed"
                } else {
                    "failed"
                };
                let _ = self
                    .mark_card_status_and_emit(
                        &card,
                        target_status,
                        "No AI session ID found",
                    )
                    .await;
                continue;
            };

            let session_url = format!("{}/session/{}", self.opencode_url, session_id);
            let (is_stuck, reason) = match self.http_client.get(&session_url).send().await {
                Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => {
                    (true, "OpenCode session not found".to_string())
                }
                Ok(response) if !response.status().is_success() => (false, String::new()),
                Ok(response) => match response.json::<serde_json::Value>().await {
                    Ok(body) => {
                        let status = body
                            .get("status")
                            .and_then(|status| {
                                if status.is_null() {
                                    return None;
                                }
                                status
                                    .get("type")
                                    .and_then(serde_json::Value::as_str)
                                    .or_else(|| status.as_str())
                            })
                            .unwrap_or("idle");
                        if status == "busy" {
                            (false, String::new())
                        } else {
                            (true, format!("Session status: {status}"))
                        }
                    }
                    Err(error) => (true, format!("Failed to decode session: {error}")),
                },
                Err(error) => (true, format!("Failed to reach OpenCode: {error}")),
            };

            if !is_stuck {
                continue;
            }

            if let Some(tool_state) = self.check_session_has_running_tool(session_id).await {
                tracing::info!(
                    card_id = card.id,
                    session_id,
                    tool = tool_state,
                    "Card has running tool call — marking as waiting instead of failed"
                );
                let now = Utc::now().to_rfc3339();
                let mut progress: serde_json::Value =
                    serde_json::from_str(&card.ai_progress).unwrap_or_else(|_| json!({}));
                progress["waiting_since"] = json!(now);
                progress["waiting_tool"] = json!(tool_state);
                let _ = sqlx::query(
                    "UPDATE cards SET ai_status = 'waiting', ai_progress = ?, updated_at = ? WHERE id = ?",
                )
                .bind(progress.to_string())
                .bind(&now)
                .bind(&card.id)
                .execute(&self.db)
                .await;

                let event = WsEvent::AiStatusChanged {
                    card_id: card.id.clone(),
                    board_id: card.board_id.clone(),
                    status: "waiting".to_string(),
                    progress,
                    stage: card.stage.clone(),
                    ai_session_id: card.ai_session_id.clone(),
                };
                if let Ok(payload) = serde_json::to_string(&event) {
                    let _ = self.sse_tx.send(payload);
                }
                continue;
            }

            let target_status = if card.stage == "review" {
                "completed"
            } else {
                "failed"
            };

            let full_reason = format!(
                "{reason}; last updated {} min ago",
                (Utc::now() - updated_at).num_minutes()
            );

            if let Err(error) = self
                .mark_card_status_and_emit(&card, target_status, &full_reason)
                .await
            {
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
                target_status,
                reason = full_reason.as_str(),
                "Recovered stuck card"
            );
        }

        Ok(())
    }

    async fn check_session_has_running_tool(&self, session_id: &str) -> Option<String> {
        let messages_url = format!(
            "{}/session/{}/message",
            self.opencode_url, session_id
        );
        let response = self.http_client.get(&messages_url).send().await.ok()?;
        let msgs: Vec<serde_json::Value> = response.json().await.ok()?;
        let last = msgs.last()?;
        for part in last.get("parts")?.as_array()? {
            if part.get("type").and_then(|t| t.as_str()) == Some("tool") {
                let state = part.get("state").and_then(|s| s.as_object())?;
                let status = state.get("status").and_then(|s| s.as_str())?;
                if status == "running" || status == "pending" {
                    let tool_name = part
                        .get("tool")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");
                    return Some(tool_name.to_string());
                }
            }
        }
        None
    }

    async fn check_waiting_card(&self, card: &Card) {
        let Some(session_id) = card.ai_session_id.as_deref() else {
            return;
        };
        if self.check_session_has_running_tool(session_id).await.is_some() {
            return;
        }
        let session_url = format!("{}/session/{}", self.opencode_url, session_id);
        let is_busy = match self.http_client.get(&session_url).send().await {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(body) => {
                    let status = body
                        .get("status")
                        .and_then(|s| {
                            if s.is_null() { None } else { s.get("type").and_then(|t| t.as_str()).or_else(|| s.as_str()) }
                        })
                        .unwrap_or("idle");
                    status == "busy"
                }
                _ => false,
            },
            _ => return,
        };
        if is_busy {
            let now = Utc::now().to_rfc3339();
            let _ = sqlx::query("UPDATE cards SET ai_status = 'working', updated_at = ? WHERE id = ?")
                .bind(&now)
                .bind(&card.id)
                .execute(&self.db)
                .await;
            let event = WsEvent::AiStatusChanged {
                card_id: card.id.clone(),
                board_id: card.board_id.clone(),
                status: "working".to_string(),
                progress: serde_json::from_str(&card.ai_progress).unwrap_or_else(|_| json!({})),
                stage: card.stage.clone(),
                ai_session_id: card.ai_session_id.clone(),
            };
            if let Ok(payload) = serde_json::to_string(&event) {
                let _ = self.sse_tx.send(payload);
            }
        }
    }

    async fn mark_card_status_and_emit(
        &self,
        card: &Card,
        status: &str,
        reason: &str,
    ) -> Result<(), KanbanError> {
        let now = Utc::now().to_rfc3339();
        let mut progress: serde_json::Value =
            serde_json::from_str(&card.ai_progress).unwrap_or_else(|_| json!({}));
        if status == "failed" {
            progress["failure_reason"] = json!(reason);
            progress["failed_at"] = json!(now);
        }

        sqlx::query(
            "UPDATE cards SET ai_status = ?, ai_progress = ?, updated_at = ? WHERE id = ?",
        )
        .bind(status)
        .bind(progress.to_string())
        .bind(&now)
        .bind(&card.id)
        .execute(&self.db)
        .await?;

        let event = WsEvent::AiStatusChanged {
            card_id: card.id.to_string(),
            board_id: card.board_id.clone(),
            status: status.to_string(),
            progress,
            stage: card.stage.clone(),
            ai_session_id: card.ai_session_id.clone(),
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
