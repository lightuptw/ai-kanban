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

        let session_response = match self
            .http_client
            .post(format!("{}/session", self.opencode_url))
            .json(&json!({"working_directory": card.working_directory}))
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

        let prompt = format!(
            "A work plan has been generated at {}. Read it carefully, then execute /start-work to begin. Work through ALL TODOs systematically.",
            plan_path
        );

        let prompt_response = match self
            .http_client
            .post(format!("{}/session/{}/prompt_async", self.opencode_url, &session_id))
            .json(&json!({"prompt": prompt}))
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                tracing::warn!(card_id = card.id, session_id = session_id.as_str(), error = %err, "Failed to send /start-work prompt");
                Self::mark_failed_with_plan(db, &card.id, &plan_path).await?;
                return Ok(String::new());
            }
        };

        if !prompt_response.status().is_success() {
            tracing::warn!(
                card_id = card.id,
                session_id = session_id.as_str(),
                status = %prompt_response.status(),
                "OpenCode prompt_async returned non-success status"
            );
            Self::mark_failed_with_plan(db, &card.id, &plan_path).await?;
            return Ok(String::new());
        }

        sqlx::query("UPDATE cards SET ai_session_id = ?, ai_status = ?, plan_path = ?, updated_at = ? WHERE id = ?")
            .bind(&session_id)
            .bind("dispatched")
            .bind(&plan_path)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(&card.id)
            .execute(db)
            .await?;

        Ok(session_id)
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
