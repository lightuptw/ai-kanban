use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::api::handlers::sse::WsEvent;
use crate::domain::{KanbanError, Notification, NotificationType};

pub struct NotificationService;

impl NotificationService {
    pub async fn create_notification(
        pool: &SqlitePool,
        sse_tx: &broadcast::Sender<String>,
        user_id: Option<&str>,
        notification_type: NotificationType,
        title: &str,
        message: &str,
        card_id: Option<&str>,
        board_id: Option<&str>,
    ) -> Result<Notification, KanbanError> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO notifications (id, user_id, notification_type, title, message, card_id, board_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(notification_type.to_string())
        .bind(title)
        .bind(message)
        .bind(card_id)
        .bind(board_id)
        .bind(&created_at)
        .execute(pool)
        .await?;

        let notification: Notification =
            sqlx::query_as("SELECT * FROM notifications WHERE id = ?")
                .bind(&id)
                .fetch_one(pool)
                .await?;

        let event = WsEvent::NotificationCreated {
            notification: serde_json::to_value(&notification).unwrap_or_default(),
        };
        if let Ok(payload) = serde_json::to_string(&event) {
            let _ = sse_tx.send(payload);
        }

        Ok(notification)
    }

    pub async fn list_notifications(
        pool: &SqlitePool,
        user_id: Option<&str>,
        unread_only: bool,
    ) -> Result<Vec<Notification>, KanbanError> {
        let notifications = match (user_id, unread_only) {
            (Some(uid), true) => {
                sqlx::query_as::<_, Notification>(
                    "SELECT * FROM notifications WHERE user_id = ? AND is_read = 0 ORDER BY created_at DESC",
                )
                .bind(uid)
                .fetch_all(pool)
                .await?
            }
            (Some(uid), false) => {
                sqlx::query_as::<_, Notification>(
                    "SELECT * FROM notifications WHERE user_id = ? ORDER BY created_at DESC",
                )
                .bind(uid)
                .fetch_all(pool)
                .await?
            }
            (None, true) => {
                sqlx::query_as::<_, Notification>(
                    "SELECT * FROM notifications WHERE is_read = 0 ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await?
            }
            (None, false) => {
                sqlx::query_as::<_, Notification>(
                    "SELECT * FROM notifications ORDER BY created_at DESC",
                )
                .fetch_all(pool)
                .await?
            }
        };

        Ok(notifications)
    }

    pub async fn mark_read(pool: &SqlitePool, id: &str) -> Result<Notification, KanbanError> {
        let result = sqlx::query("UPDATE notifications SET is_read = 1 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(KanbanError::NotFound(format!(
                "Notification not found: {}",
                id
            )));
        }

        let notification: Notification = sqlx::query_as("SELECT * FROM notifications WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await?;

        Ok(notification)
    }

    pub async fn mark_all_read(pool: &SqlitePool, user_id: Option<&str>) -> Result<u64, KanbanError> {
        let result = if let Some(uid) = user_id {
            sqlx::query("UPDATE notifications SET is_read = 1 WHERE is_read = 0 AND user_id = ?")
                .bind(uid)
                .execute(pool)
                .await?
        } else {
            sqlx::query("UPDATE notifications SET is_read = 1 WHERE is_read = 0")
                .execute(pool)
                .await?
        };

        Ok(result.rows_affected())
    }

    pub async fn delete_notification(pool: &SqlitePool, id: &str) -> Result<(), KanbanError> {
        let result = sqlx::query("DELETE FROM notifications WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(KanbanError::NotFound(format!(
                "Notification not found: {}",
                id
            )));
        }

        Ok(())
    }
}
