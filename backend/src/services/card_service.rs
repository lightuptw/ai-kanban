use chrono::Utc;
use serde_json::json;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::api::dto::{
    BoardResponse, CardResponse, CardSummary, CreateCardRequest, CreateCommentRequest,
    CreateSubtaskRequest, MoveCardRequest, UpdateCardRequest, UpdateSubtaskRequest,
};
use crate::domain::{Card, Comment, KanbanError, Label, Stage, Subtask};

pub struct CardService;

impl CardService {
    pub async fn save_card_version_snapshot(
        pool: &SqlitePool,
        card: &Card,
        changed_by: &str,
    ) -> Result<(), KanbanError> {
        let snapshot = json!({
            "title": &card.title,
            "description": &card.description,
            "priority": &card.priority,
            "stage": &card.stage,
            "working_directory": &card.working_directory,
            "linked_documents": &card.linked_documents,
        });

        let version_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO card_versions (id, card_id, snapshot, changed_by, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&version_id)
        .bind(&card.id)
        .bind(snapshot.to_string())
        .bind(changed_by)
        .bind(&now)
        .execute(pool)
        .await?;

        sqlx::query(
            "DELETE FROM card_versions WHERE id IN (SELECT id FROM card_versions WHERE card_id = ? ORDER BY created_at DESC LIMIT -1 OFFSET 50)",
        )
        .bind(&card.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    // ── Card CRUD ──────────────────────────────────────────────

    pub async fn create_card(
        pool: &SqlitePool,
        req: CreateCardRequest,
    ) -> Result<CardResponse, KanbanError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let stage = req.stage.unwrap_or_else(|| "backlog".into());
        let description = req.description.unwrap_or_default();
        let priority = req.priority.unwrap_or_else(|| "medium".into());
        let working_directory = req.working_directory.unwrap_or_else(|| ".".into());
        let board_id = req.board_id.unwrap_or_else(|| "default".into());

        // Validate stage
        stage
            .parse::<Stage>()
            .map_err(KanbanError::BadRequest)?;

        // New cards get position = max_position + 1000
        let row = sqlx::query("SELECT COALESCE(MAX(position), 0) as max_pos FROM cards WHERE stage = ?")
            .bind(&stage)
            .fetch_one(pool)
            .await?;
        let max_pos: i64 = row.get("max_pos");
        let position = max_pos + 1000;

        sqlx::query(
            "INSERT INTO cards (id, title, description, stage, position, priority, working_directory, board_id, ai_status, ai_progress, linked_documents, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'idle', '{}', '[]', ?, ?)"
        )
        .bind(&id)
        .bind(&req.title)
        .bind(&description)
        .bind(&stage)
        .bind(position)
        .bind(&priority)
        .bind(&working_directory)
        .bind(&board_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Self::get_card_by_id(pool, &id).await
    }

    pub async fn get_card_by_id(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<CardResponse, KanbanError> {
        let card = Self::get_card_model(pool, id).await?;

        let subtasks: Vec<Subtask> =
            sqlx::query_as("SELECT * FROM subtasks WHERE card_id = ? ORDER BY phase ASC, position ASC")
                .bind(id)
                .fetch_all(pool)
                .await?;

        let labels: Vec<Label> = sqlx::query_as(
            "SELECT l.id, l.name, l.color FROM labels l JOIN card_labels cl ON l.id = cl.label_id WHERE cl.card_id = ?",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        let comments: Vec<Comment> =
            sqlx::query_as("SELECT * FROM comments WHERE card_id = ? ORDER BY created_at ASC")
                .bind(id)
                .fetch_all(pool)
                .await?;

        Ok(CardResponse::from_card(card, subtasks, labels, comments))
    }

    pub async fn get_card_model(pool: &SqlitePool, id: &str) -> Result<Card, KanbanError> {
        let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| KanbanError::NotFound(format!("Card not found: {}", id)))?;

        Ok(card)
    }

    pub async fn get_board(pool: &SqlitePool, board_id: Option<&str>) -> Result<BoardResponse, KanbanError> {
        let query = if board_id.is_some() {
            r#"
            SELECT
                c.id, c.title, c.description, c.stage, c.position, c.priority,
                c.ai_status, c.ai_agent, c.created_at, c.updated_at,
                COALESCE((SELECT COUNT(*) FROM subtasks s WHERE s.card_id = c.id), 0) as subtask_count,
                COALESCE((SELECT COUNT(*) FROM subtasks s WHERE s.card_id = c.id AND s.completed = 1), 0) as subtask_completed,
                COALESCE((SELECT COUNT(*) FROM card_labels cl WHERE cl.card_id = c.id), 0) as label_count,
                COALESCE((SELECT COUNT(*) FROM comments co WHERE co.card_id = c.id), 0) as comment_count
            FROM cards c
            WHERE c.board_id = ?
            ORDER BY c.position ASC
            "#
        } else {
            r#"
            SELECT
                c.id, c.title, c.description, c.stage, c.position, c.priority,
                c.ai_status, c.ai_agent, c.created_at, c.updated_at,
                COALESCE((SELECT COUNT(*) FROM subtasks s WHERE s.card_id = c.id), 0) as subtask_count,
                COALESCE((SELECT COUNT(*) FROM subtasks s WHERE s.card_id = c.id AND s.completed = 1), 0) as subtask_completed,
                COALESCE((SELECT COUNT(*) FROM card_labels cl WHERE cl.card_id = c.id), 0) as label_count,
                COALESCE((SELECT COUNT(*) FROM comments co WHERE co.card_id = c.id), 0) as comment_count
            FROM cards c
            ORDER BY c.position ASC
            "#
        };

        let rows = if let Some(bid) = board_id {
            sqlx::query(query).bind(bid).fetch_all(pool).await?
        } else {
            sqlx::query(query).fetch_all(pool).await?
        };

        let mut board = BoardResponse {
            backlog: vec![],
            plan: vec![],
            todo: vec![],
            in_progress: vec![],
            review: vec![],
            done: vec![],
        };

        for row in rows {
            let stage: String = row.get("stage");
            let summary = CardSummary {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                stage: stage.clone(),
                position: row.get("position"),
                priority: row.get("priority"),
                ai_status: row.get("ai_status"),
                ai_agent: row.get("ai_agent"),
                subtask_count: row.get("subtask_count"),
                subtask_completed: row.get("subtask_completed"),
                label_count: row.get("label_count"),
                comment_count: row.get("comment_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };

            match stage.as_str() {
                "backlog" => board.backlog.push(summary),
                "plan" => board.plan.push(summary),
                "todo" => board.todo.push(summary),
                "in_progress" => board.in_progress.push(summary),
                "review" => board.review.push(summary),
                "done" => board.done.push(summary),
                _ => board.backlog.push(summary),
            }
        }

        Ok(board)
    }

    pub async fn update_card(
        pool: &SqlitePool,
        id: &str,
        req: UpdateCardRequest,
    ) -> Result<CardResponse, KanbanError> {
        let existing: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| KanbanError::NotFound(format!("Card not found: {}", id)))?;
        let existing_for_snapshot = existing.clone();

        let now = Utc::now().to_rfc3339();
        let title = req.title.unwrap_or(existing.title);
        let description = req.description.unwrap_or(existing.description);
        let stage = req.stage.unwrap_or(existing.stage);
        let position = req.position.unwrap_or(existing.position);
        let priority = req.priority.unwrap_or(existing.priority);
        let working_directory = req.working_directory.unwrap_or(existing.working_directory);
        let linked_documents = req.linked_documents.unwrap_or(existing.linked_documents);
        let ai_agent = match &req.ai_agent {
            Some(s) if s.is_empty() => None,
            Some(s) => Some(s.clone()),
            None => existing.ai_agent,
        };

        stage
            .parse::<Stage>()
            .map_err(KanbanError::BadRequest)?;

        Self::save_card_version_snapshot(pool, &existing_for_snapshot, "user").await?;

        sqlx::query(
            "UPDATE cards SET title = ?, description = ?, stage = ?, position = ?, priority = ?, working_directory = ?, linked_documents = ?, ai_agent = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&title)
        .bind(&description)
        .bind(&stage)
        .bind(position)
        .bind(&priority)
        .bind(&working_directory)
        .bind(&linked_documents)
        .bind(&ai_agent)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Self::get_card_by_id(pool, id).await
    }

    pub async fn move_card(
        pool: &SqlitePool,
        id: &str,
        req: MoveCardRequest,
    ) -> Result<CardResponse, KanbanError> {
        // Validate stage
        let new_stage = req
            .stage
            .parse::<Stage>()
            .map_err(KanbanError::BadRequest)?;

        // Check card exists and get current stage
        let existing: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| KanbanError::NotFound(format!("Card not found: {}", id)))?;

        let now = Utc::now().to_rfc3339();

        let position = match req.position {
            Some(pos) => pos,
            None => {
                // Append to end of target stage: max + 1000
                let row = sqlx::query(
                    "SELECT COALESCE(MAX(position), 0) as max_pos FROM cards WHERE stage = ?",
                )
                .bind(new_stage.as_str())
                .fetch_one(pool)
                .await?;
                let max_pos: i64 = row.get("max_pos");
                max_pos + 1000
            }
        };

        sqlx::query("UPDATE cards SET stage = ?, position = ?, updated_at = ? WHERE id = ?")
            .bind(new_stage.as_str())
            .bind(position)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        // Log if moved to "todo" — AI dispatch trigger point (Task 10)
        if new_stage == Stage::Todo && existing.stage != "todo" {
            tracing::info!(
                card_id = id,
                from_stage = existing.stage.as_str(),
                "Card moved to 'todo' stage — AI dispatch trigger point"
            );
        }

        Self::get_card_by_id(pool, id).await
    }

    pub async fn delete_card(pool: &SqlitePool, id: &str) -> Result<(), KanbanError> {
        let result = sqlx::query("DELETE FROM cards WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(KanbanError::NotFound(format!("Card not found: {}", id)));
        }

        Ok(())
    }

    // ── Subtask Operations ─────────────────────────────────────

    pub async fn create_subtask(
        pool: &SqlitePool,
        card_id: &str,
        req: CreateSubtaskRequest,
    ) -> Result<Subtask, KanbanError> {
        // Verify card exists
        let _: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(card_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| KanbanError::NotFound(format!("Card not found: {}", card_id)))?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let row = sqlx::query(
            "SELECT COALESCE(MAX(position), 0) as max_pos FROM subtasks WHERE card_id = ?",
        )
        .bind(card_id)
        .fetch_one(pool)
        .await?;
        let max_pos: i64 = row.get("max_pos");
        let position = max_pos + 1000;

        sqlx::query(
            "INSERT INTO subtasks (id, card_id, title, completed, position, phase, phase_order, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(card_id)
        .bind(&req.title)
        .bind(position)
        .bind(&req.phase)
        .bind(req.phase_order)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        let subtask: Subtask = sqlx::query_as("SELECT * FROM subtasks WHERE id = ?")
            .bind(&id)
            .fetch_one(pool)
            .await?;

        Ok(subtask)
    }

    pub async fn get_subtasks(pool: &SqlitePool, card_id: &str) -> Result<Vec<Subtask>, KanbanError> {
        let subtasks: Vec<Subtask> =
            sqlx::query_as("SELECT * FROM subtasks WHERE card_id = ? ORDER BY phase ASC, position ASC")
                .bind(card_id)
                .fetch_all(pool)
                .await?;

        Ok(subtasks)
    }

    pub async fn update_subtask(
        pool: &SqlitePool,
        id: &str,
        req: UpdateSubtaskRequest,
    ) -> Result<Subtask, KanbanError> {
        let existing: Subtask = sqlx::query_as("SELECT * FROM subtasks WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| KanbanError::NotFound(format!("Subtask not found: {}", id)))?;

        let now = Utc::now().to_rfc3339();
        let title = req.title.unwrap_or(existing.title);
        let completed = req.completed.unwrap_or(existing.completed);
        let phase = req.phase.unwrap_or(existing.phase);
        let phase_order = req.phase_order.unwrap_or(existing.phase_order);
        let position = req.position.unwrap_or(existing.position);

        sqlx::query("UPDATE subtasks SET title = ?, completed = ?, phase = ?, phase_order = ?, position = ?, updated_at = ? WHERE id = ?")
            .bind(&title)
            .bind(completed)
            .bind(&phase)
            .bind(phase_order)
            .bind(position)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        let subtask: Subtask = sqlx::query_as("SELECT * FROM subtasks WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await?;

        Ok(subtask)
    }

    pub async fn delete_subtask(pool: &SqlitePool, id: &str) -> Result<(), KanbanError> {
        let result = sqlx::query("DELETE FROM subtasks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(KanbanError::NotFound(format!(
                "Subtask not found: {}",
                id
            )));
        }

        Ok(())
    }

    // ── Comment Operations ─────────────────────────────────────

    pub async fn create_comment(
        pool: &SqlitePool,
        card_id: &str,
        req: CreateCommentRequest,
    ) -> Result<Comment, KanbanError> {
        // Verify card exists
        let _: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(card_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| KanbanError::NotFound(format!("Card not found: {}", card_id)))?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let author = req.author.unwrap_or_else(|| "user".into());

        sqlx::query(
            "INSERT INTO comments (id, card_id, author, content, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(card_id)
        .bind(&author)
        .bind(&req.content)
        .bind(&now)
        .execute(pool)
        .await?;

        let comment: Comment = sqlx::query_as("SELECT * FROM comments WHERE id = ?")
            .bind(&id)
            .fetch_one(pool)
            .await?;

        Ok(comment)
    }

    // ── Label Operations ───────────────────────────────────────

    pub async fn list_labels(pool: &SqlitePool) -> Result<Vec<Label>, KanbanError> {
        let labels: Vec<Label> = sqlx::query_as("SELECT * FROM labels ORDER BY name ASC")
            .fetch_all(pool)
            .await?;

        Ok(labels)
    }

    pub async fn add_label_to_card(
        pool: &SqlitePool,
        card_id: &str,
        label_id: &str,
    ) -> Result<(), KanbanError> {
        // Verify card exists
        let _: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(card_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| KanbanError::NotFound(format!("Card not found: {}", card_id)))?;

        // Verify label exists
        let _: Label = sqlx::query_as("SELECT * FROM labels WHERE id = ?")
            .bind(label_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| KanbanError::NotFound(format!("Label not found: {}", label_id)))?;

        // Insert (ignore duplicate)
        sqlx::query("INSERT OR IGNORE INTO card_labels (card_id, label_id) VALUES (?, ?)")
            .bind(card_id)
            .bind(label_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn remove_label_from_card(
        pool: &SqlitePool,
        card_id: &str,
        label_id: &str,
    ) -> Result<(), KanbanError> {
        let result =
            sqlx::query("DELETE FROM card_labels WHERE card_id = ? AND label_id = ?")
                .bind(card_id)
                .bind(label_id)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(KanbanError::NotFound(
                "Card-label association not found".into(),
            ));
        }

        Ok(())
    }
}
