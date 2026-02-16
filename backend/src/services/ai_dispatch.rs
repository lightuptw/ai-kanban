use std::path::Path;

use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::domain::{Card, KanbanError, Subtask};

use super::plan_generator::PlanGenerator;

pub struct AiDispatchService {
    http_client: reqwest::Client,
    opencode_url: String,
}

impl AiDispatchService {
    pub fn new(http_client: reqwest::Client, opencode_url: String) -> Self {
        Self {
            http_client,
            opencode_url,
        }
    }

    pub async fn dispatch_card(
        &self,
        card: &Card,
        subtasks: &[Subtask],
        db: &SqlitePool,
    ) -> Result<String, KanbanError> {
        if !Path::new(&card.working_directory).exists() {
            tracing::warn!(
                card_id = card.id,
                working_directory = card.working_directory,
                "Working directory does not exist; marking card as failed"
            );
            Self::mark_failed(db, &card.id).await?;
            return Ok(String::new());
        }

        let plan_content = PlanGenerator::generate_plan(card, subtasks)
            .map_err(KanbanError::OpenCodeError)?;
        let plan_path = PlanGenerator::write_plan_file(&card.working_directory, &card.title, &plan_content)
            .map_err(KanbanError::OpenCodeError)?;

        // Wake up opencode server (it may be sleeping)
        let _ = self
            .http_client
            .get(format!("{}/health", self.opencode_url))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;

        let session_response = match self
            .http_client
            .post(format!("{}/session", self.opencode_url))
            .json(&json!({}))
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                tracing::warn!(card_id = card.id, error = %err, "Failed to create OpenCode session");
                Self::mark_failed_with_plan(db, &card.id, &plan_path).await?;
                return Ok(String::new());
            }
        };

        if !session_response.status().is_success() {
            tracing::warn!(
                card_id = card.id,
                status = %session_response.status(),
                "OpenCode session creation returned non-success status"
            );
            Self::mark_failed_with_plan(db, &card.id, &plan_path).await?;
            return Ok(String::new());
        }

        let body = match session_response.json::<Value>().await {
            Ok(value) => value,
            Err(err) => {
                tracing::warn!(card_id = card.id, error = %err, "Failed to decode OpenCode session response");
                Self::mark_failed_with_plan(db, &card.id, &plan_path).await?;
                return Ok(String::new());
            }
        };

        let session_id = match body
            .get("id")
            .and_then(Value::as_str)
        {
            Some(id) => id.to_string(),
            None => {
                tracing::warn!(card_id = card.id, "OpenCode session response missing id");
                Self::mark_failed_with_plan(db, &card.id, &plan_path).await?;
                return Ok(String::new());
            }
        };

        // Save session_id immediately (before sending the message, which blocks)
        sqlx::query("UPDATE cards SET ai_session_id = ?, ai_status = ?, plan_path = ?, updated_at = ? WHERE id = ?")
            .bind(&session_id)
            .bind("dispatched")
            .bind(&plan_path)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(&card.id)
            .execute(db)
            .await?;

        // Send the work plan message in a background task.
        // The /session/{id}/message endpoint is synchronous (blocks until AI finishes),
        // so we fire-and-forget. The SSE relay will track progress via opencode events.
        let agent_instruction = if let Some(agent) = &card.ai_agent {
            format!("You are acting as the {} agent. ", agent)
        } else {
            String::new()
        };
        let prompt = format!(
            "{}A work plan has been generated at {}. Read it carefully, then execute /start-work to begin. Work through ALL TODOs systematically.",
            agent_instruction, plan_path
        );
        let http_client = self.http_client.clone();
        let message_url = format!("{}/session/{}/message", self.opencode_url, &session_id);
        let card_id = card.id.clone();
        let db_clone = db.clone();
        let session_id_clone = session_id.clone();

        tokio::spawn(async move {
            tracing::info!(card_id = card_id.as_str(), session_id = session_id_clone.as_str(), "Sending work plan to OpenCode agent");

            let result = http_client
                .post(&message_url)
                .json(&json!({"parts": [{"type": "text", "text": prompt}]}))
                .send()
                .await;

            match result {
                Ok(response) if response.status().is_success() => {
                    tracing::info!(card_id = card_id.as_str(), "OpenCode agent message sent successfully");
                }
                Ok(response) => {
                    tracing::warn!(card_id = card_id.as_str(), status = %response.status(), "OpenCode message returned non-success");
                    if let Err(e) = Self::mark_failed(&db_clone, &card_id).await {
                        tracing::warn!(error = %e, card_id = card_id.as_str(), "Failed to mark card as failed after non-success OpenCode response");
                    }
                }
                Err(err) => {
                    tracing::warn!(card_id = card_id.as_str(), error = %err, "Failed to send work plan message");
                    if let Err(e) = Self::mark_failed(&db_clone, &card_id).await {
                        tracing::warn!(error = %e, card_id = card_id.as_str(), "Failed to mark card as failed after message send error");
                    }
                }
            }
        });

        Ok(session_id.to_string())
    }

    pub async fn abort_session(&self, session_id: &str) -> Result<(), KanbanError> {
        let response = self
            .http_client
            .post(format!("{}/session/{}/abort", self.opencode_url, session_id))
            .send()
            .await
            .map_err(|e| KanbanError::OpenCodeError(format!("Failed to abort session: {}", e)))?;

        if !response.status().is_success() {
            return Err(KanbanError::OpenCodeError(format!(
                "OpenCode abort failed with status {}",
                response.status()
            )));
        }

        Ok(())
    }

    async fn mark_failed(db: &SqlitePool, card_id: &str) -> Result<(), KanbanError> {
        sqlx::query("UPDATE cards SET ai_status = ?, updated_at = ? WHERE id = ?")
            .bind("failed")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(card_id)
            .execute(db)
            .await?;

        Ok(())
    }

    async fn mark_failed_with_plan(
        db: &SqlitePool,
        card_id: &str,
        plan_path: &str,
    ) -> Result<(), KanbanError> {
        sqlx::query("UPDATE cards SET ai_status = ?, plan_path = ?, updated_at = ? WHERE id = ?")
            .bind("failed")
            .bind(plan_path)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(card_id)
            .execute(db)
            .await?;

        Ok(())
    }
}
