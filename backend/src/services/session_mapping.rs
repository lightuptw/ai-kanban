use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;

use crate::domain::SessionMapping;

pub struct SessionMappingService;

impl SessionMappingService {
    pub async fn insert(
        db: &SqlitePool,
        child_session_id: &str,
        card_id: &str,
        parent_session_id: &str,
        agent_type: Option<&str>,
        description: &str,
    ) -> Result<SessionMapping> {
        let created_at = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT OR IGNORE INTO session_mappings (child_session_id, card_id, parent_session_id, agent_type, description, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(child_session_id)
        .bind(card_id)
        .bind(parent_session_id)
        .bind(agent_type)
        .bind(description)
        .bind(&created_at)
        .execute(db)
        .await?;

        Ok(SessionMapping {
            child_session_id: child_session_id.to_string(),
            card_id: card_id.to_string(),
            parent_session_id: parent_session_id.to_string(),
            agent_type: agent_type.map(str::to_owned),
            description: description.to_string(),
            created_at,
        })
    }

    pub async fn find_card_by_child_session(
        db: &SqlitePool,
        child_session_id: &str,
    ) -> Result<Option<String>> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT card_id FROM session_mappings WHERE child_session_id = ?",
        )
        .bind(child_session_id)
        .fetch_optional(db)
        .await?;

        Ok(result.map(|(id,)| id))
    }

    pub async fn list_for_card(
        db: &SqlitePool,
        card_id: &str,
    ) -> Result<Vec<SessionMapping>> {
        let mappings: Vec<SessionMapping> = sqlx::query_as(
            "SELECT * FROM session_mappings WHERE card_id = ? ORDER BY created_at ASC",
        )
        .bind(card_id)
        .fetch_all(db)
        .await?;

        Ok(mappings)
    }

    pub async fn delete_for_card(db: &SqlitePool, card_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM session_mappings WHERE card_id = ?")
            .bind(card_id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn get_agent_type(
        db: &SqlitePool,
        child_session_id: &str,
    ) -> Result<Option<String>> {
        let result: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT agent_type FROM session_mappings WHERE child_session_id = ?",
        )
        .bind(child_session_id)
        .fetch_optional(db)
        .await?;

        Ok(result.and_then(|(t,)| t))
    }
}
