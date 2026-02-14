use chrono::Utc;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::*,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{Card, Comment, Label, Stage, Subtask};

#[derive(Clone)]
pub struct KanbanMcp {
    pool: SqlitePool,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Serialize, FromRow)]
struct Board {
    id: String,
    name: String,
    position: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct CardSummary {
    id: String,
    title: String,
    description: String,
    stage: String,
    position: i64,
    priority: String,
    ai_status: String,
    subtask_count: i64,
    subtask_completed: i64,
    label_count: i64,
    comment_count: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct BoardCards {
    backlog: Vec<CardSummary>,
    plan: Vec<CardSummary>,
    todo: Vec<CardSummary>,
    in_progress: Vec<CardSummary>,
    review: Vec<CardSummary>,
    done: Vec<CardSummary>,
}

#[derive(Debug, Serialize)]
struct CardDetail {
    card: Card,
    subtasks: Vec<Subtask>,
    comments: Vec<Comment>,
    labels: Vec<Label>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateBoardInput {
    name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteBoardInput {
    board_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetBoardCardsInput {
    board_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetCardInput {
    card_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateCardInput {
    title: String,
    description: Option<String>,
    stage: Option<String>,
    priority: Option<String>,
    board_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateCardInput {
    card_id: String,
    title: Option<String>,
    description: Option<String>,
    stage: Option<String>,
    priority: Option<String>,
    working_directory: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteCardInput {
    card_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateSubtaskInput {
    card_id: String,
    title: String,
    phase: Option<String>,
    phase_order: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateSubtaskInput {
    subtask_id: String,
    title: Option<String>,
    completed: Option<bool>,
    phase: Option<String>,
    phase_order: Option<i64>,
    position: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteSubtaskInput {
    subtask_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetCommentsInput {
    card_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddCommentInput {
    card_id: String,
    content: String,
    author: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateCommentInput {
    comment_id: String,
    content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteCommentInput {
    comment_id: String,
}

#[tool_router]
impl KanbanMcp {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            tool_router: Self::tool_router(),
        }
    }

    fn db_err(e: sqlx::Error) -> McpError {
        McpError::internal_error(format!("Database error: {}", e), None)
    }

    fn json_result<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
        let text = serde_json::to_string_pretty(value)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    fn parse_stage(stage: &str) -> Result<(), McpError> {
        stage
            .parse::<Stage>()
            .map(|_| ())
            .map_err(|e| McpError::invalid_params(e, None))
    }

    #[tool(
        description = "List all boards ordered by position. Use this to discover available boards before creating or fetching board-specific cards. Returns a JSON array of board objects with id, name, position, created_at, and updated_at."
    )]
    async fn kanban_list_boards(&self) -> Result<CallToolResult, McpError> {
        let boards: Vec<Board> = sqlx::query_as(
            "SELECT id, name, position, created_at, updated_at FROM boards ORDER BY position ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_err)?;

        Self::json_result(&boards)
    }

    #[tool(
        description = "Create a new board. Use this when you need a separate workspace for a project stream. Returns the created board as JSON with id, name, position, created_at, and updated_at."
    )]
    async fn kanban_create_board(
        &self,
        Parameters(input): Parameters<CreateBoardInput>,
    ) -> Result<CallToolResult, McpError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let max_pos: Option<(i64,)> = sqlx::query_as("SELECT COALESCE(MAX(position), 0) FROM boards")
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::db_err)?;
        let position = max_pos.map(|row| row.0).unwrap_or(0) + 1000;

        let board: Board = sqlx::query_as(
            "INSERT INTO boards (id, name, position, created_at, updated_at) VALUES (?, ?, ?, ?, ?) RETURNING id, name, position, created_at, updated_at",
        )
        .bind(&id)
        .bind(&input.name)
        .bind(position)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await
        .map_err(Self::db_err)?;

        Self::json_result(&board)
    }

    #[tool(
        description = "Delete a board by id. Use this when cleaning up unused boards. Returns a JSON object confirming deletion with the deleted board id."
    )]
    async fn kanban_delete_board(
        &self,
        Parameters(input): Parameters<DeleteBoardInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = sqlx::query("DELETE FROM boards WHERE id = ?")
            .bind(&input.board_id)
            .execute(&self.pool)
            .await
            .map_err(Self::db_err)?;

        if result.rows_affected() == 0 {
            return Err(McpError::invalid_params(
                format!("Board not found: {}", input.board_id),
                None,
            ));
        }

        Self::json_result(&json!({"deleted": true, "board_id": input.board_id}))
    }

    #[tool(
        description = "Fetch board cards grouped by workflow stage. Use this as the primary board overview call before planning or acting on tasks. Returns a JSON object with keys backlog, plan, todo, in_progress, review, and done, each containing card summary arrays."
    )]
    async fn kanban_get_board_cards(
        &self,
        Parameters(input): Parameters<GetBoardCardsInput>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = input.board_id.unwrap_or_else(|| "default".to_string());
        let rows = sqlx::query(
            r#"
            SELECT
                c.id, c.title, c.description, c.stage, c.position, c.priority,
                c.ai_status, c.created_at, c.updated_at,
                COALESCE((SELECT COUNT(*) FROM subtasks s WHERE s.card_id = c.id), 0) as subtask_count,
                COALESCE((SELECT COUNT(*) FROM subtasks s WHERE s.card_id = c.id AND s.completed = 1), 0) as subtask_completed,
                COALESCE((SELECT COUNT(*) FROM card_labels cl WHERE cl.card_id = c.id), 0) as label_count,
                COALESCE((SELECT COUNT(*) FROM comments co WHERE co.card_id = c.id), 0) as comment_count
            FROM cards c
            WHERE c.board_id = ?
            ORDER BY c.position ASC
            "#,
        )
        .bind(&board_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_err)?;

        let mut grouped = BoardCards {
            backlog: Vec::new(),
            plan: Vec::new(),
            todo: Vec::new(),
            in_progress: Vec::new(),
            review: Vec::new(),
            done: Vec::new(),
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
                subtask_count: row.get("subtask_count"),
                subtask_completed: row.get("subtask_completed"),
                label_count: row.get("label_count"),
                comment_count: row.get("comment_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };

            match stage.as_str() {
                "backlog" => grouped.backlog.push(summary),
                "plan" => grouped.plan.push(summary),
                "todo" => grouped.todo.push(summary),
                "in_progress" => grouped.in_progress.push(summary),
                "review" => grouped.review.push(summary),
                "done" => grouped.done.push(summary),
                _ => grouped.backlog.push(summary),
            }
        }

        Self::json_result(&grouped)
    }

    #[tool(
        description = "Get full details for one card, including subtasks, comments, and labels. Use this when you need complete context before editing or executing work. Returns a JSON object with card, subtasks, comments, and labels."
    )]
    async fn kanban_get_card(
        &self,
        Parameters(input): Parameters<GetCardInput>,
    ) -> Result<CallToolResult, McpError> {
        let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(&input.card_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::db_err)?
            .ok_or_else(|| McpError::invalid_params(format!("Card not found: {}", input.card_id), None))?;

        let subtasks: Vec<Subtask> = sqlx::query_as(
            "SELECT * FROM subtasks WHERE card_id = ? ORDER BY phase ASC, position ASC",
        )
        .bind(&input.card_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_err)?;

        let comments: Vec<Comment> =
            sqlx::query_as("SELECT * FROM comments WHERE card_id = ? ORDER BY created_at ASC")
                .bind(&input.card_id)
                .fetch_all(&self.pool)
                .await
                .map_err(Self::db_err)?;

        let labels: Vec<Label> = sqlx::query_as(
            "SELECT l.id, l.name, l.color FROM labels l JOIN card_labels cl ON l.id = cl.label_id WHERE cl.card_id = ?",
        )
        .bind(&input.card_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Self::db_err)?;

        Self::json_result(&CardDetail {
            card,
            subtasks,
            comments,
            labels,
        })
    }

    #[tool(
        description = "Create a new card on a board. Use this when starting a task, bug, or feature. Returns the created card as JSON. Defaults: stage=backlog, priority=medium, board_id=default, working_directory=."
    )]
    async fn kanban_create_card(
        &self,
        Parameters(input): Parameters<CreateCardInput>,
    ) -> Result<CallToolResult, McpError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let stage = input.stage.unwrap_or_else(|| "backlog".to_string());
        let priority = input.priority.unwrap_or_else(|| "medium".to_string());
        let description = input.description.unwrap_or_default();
        let board_id = input.board_id.unwrap_or_else(|| "default".to_string());
        let working_directory = ".";

        Self::parse_stage(&stage)?;

        let row = sqlx::query("SELECT COALESCE(MAX(position), 0) as max_pos FROM cards WHERE stage = ?")
            .bind(&stage)
            .fetch_one(&self.pool)
            .await
            .map_err(Self::db_err)?;
        let position: i64 = row.get("max_pos");
        let position = position + 1000;

        sqlx::query(
            "INSERT INTO cards (id, title, description, stage, position, priority, working_directory, board_id, ai_status, ai_progress, linked_documents, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'idle', '{}', '[]', ?, ?)",
        )
        .bind(&id)
        .bind(&input.title)
        .bind(&description)
        .bind(&stage)
        .bind(position)
        .bind(&priority)
        .bind(working_directory)
        .bind(&board_id)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(Self::db_err)?;

        let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .map_err(Self::db_err)?;

        Self::json_result(&card)
    }

    #[tool(
        description = "Update card fields by id. Use this when card details, status, or working directory change. Returns the updated card as JSON. Stage options: backlog, plan, todo, in_progress, review, done."
    )]
    async fn kanban_update_card(
        &self,
        Parameters(input): Parameters<UpdateCardInput>,
    ) -> Result<CallToolResult, McpError> {
        let existing: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(&input.card_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::db_err)?
            .ok_or_else(|| McpError::invalid_params(format!("Card not found: {}", input.card_id), None))?;

        let title = input.title.unwrap_or(existing.title);
        let description = input.description.unwrap_or(existing.description);
        let stage = input.stage.unwrap_or(existing.stage);
        let priority = input.priority.unwrap_or(existing.priority);
        let working_directory = input.working_directory.unwrap_or(existing.working_directory);
        let now = Utc::now().to_rfc3339();

        Self::parse_stage(&stage)?;

        sqlx::query(
            "UPDATE cards SET title = ?, description = ?, stage = ?, priority = ?, working_directory = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&title)
        .bind(&description)
        .bind(&stage)
        .bind(&priority)
        .bind(&working_directory)
        .bind(&now)
        .bind(&input.card_id)
        .execute(&self.pool)
        .await
        .map_err(Self::db_err)?;

        let card: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(&input.card_id)
            .fetch_one(&self.pool)
            .await
            .map_err(Self::db_err)?;

        Self::json_result(&card)
    }

    #[tool(
        description = "Delete a card by id. Use this when removing obsolete or duplicate work items. Returns a JSON object confirming deletion with the deleted card id."
    )]
    async fn kanban_delete_card(
        &self,
        Parameters(input): Parameters<DeleteCardInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = sqlx::query("DELETE FROM cards WHERE id = ?")
            .bind(&input.card_id)
            .execute(&self.pool)
            .await
            .map_err(Self::db_err)?;

        if result.rows_affected() == 0 {
            return Err(McpError::invalid_params(
                format!("Card not found: {}", input.card_id),
                None,
            ));
        }

        Self::json_result(&json!({"deleted": true, "card_id": input.card_id}))
    }

    #[tool(
        description = "Create a subtask on a card. Use this to break work into actionable checklist items. Returns the created subtask as JSON. Defaults: phase=Phase 1, phase_order=1, and position is appended within the card."
    )]
    async fn kanban_create_subtask(
        &self,
        Parameters(input): Parameters<CreateSubtaskInput>,
    ) -> Result<CallToolResult, McpError> {
        let _: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(&input.card_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::db_err)?
            .ok_or_else(|| McpError::invalid_params(format!("Card not found: {}", input.card_id), None))?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let phase = input.phase.unwrap_or_else(|| "Phase 1".to_string());
        let phase_order = input.phase_order.unwrap_or(1);

        let row = sqlx::query("SELECT COALESCE(MAX(position), 0) as max_pos FROM subtasks WHERE card_id = ?")
            .bind(&input.card_id)
            .fetch_one(&self.pool)
            .await
            .map_err(Self::db_err)?;
        let max_pos: i64 = row.get("max_pos");
        let position = max_pos + 1000;

        sqlx::query(
            "INSERT INTO subtasks (id, card_id, title, completed, position, phase, phase_order, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&input.card_id)
        .bind(&input.title)
        .bind(position)
        .bind(&phase)
        .bind(phase_order)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(Self::db_err)?;

        let subtask: Subtask = sqlx::query_as("SELECT * FROM subtasks WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .map_err(Self::db_err)?;

        Self::json_result(&subtask)
    }

    #[tool(
        description = "Update a subtask by id. Use this to rename items, check them off, or reorder/phase them. Returns the updated subtask as JSON."
    )]
    async fn kanban_update_subtask(
        &self,
        Parameters(input): Parameters<UpdateSubtaskInput>,
    ) -> Result<CallToolResult, McpError> {
        let existing: Subtask = sqlx::query_as("SELECT * FROM subtasks WHERE id = ?")
            .bind(&input.subtask_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::db_err)?
            .ok_or_else(|| McpError::invalid_params(format!("Subtask not found: {}", input.subtask_id), None))?;

        let title = input.title.unwrap_or(existing.title);
        let completed = input.completed.unwrap_or(existing.completed);
        let phase = input.phase.unwrap_or(existing.phase);
        let phase_order = input.phase_order.unwrap_or(existing.phase_order);
        let position = input.position.unwrap_or(existing.position);
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE subtasks SET title = ?, completed = ?, phase = ?, phase_order = ?, position = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&title)
        .bind(completed)
        .bind(&phase)
        .bind(phase_order)
        .bind(position)
        .bind(&now)
        .bind(&input.subtask_id)
        .execute(&self.pool)
        .await
        .map_err(Self::db_err)?;

        let subtask: Subtask = sqlx::query_as("SELECT * FROM subtasks WHERE id = ?")
            .bind(&input.subtask_id)
            .fetch_one(&self.pool)
            .await
            .map_err(Self::db_err)?;

        Self::json_result(&subtask)
    }

    #[tool(
        description = "Delete a subtask by id. Use this when a checklist item is no longer needed. Returns a JSON object confirming deletion with the deleted subtask id."
    )]
    async fn kanban_delete_subtask(
        &self,
        Parameters(input): Parameters<DeleteSubtaskInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = sqlx::query("DELETE FROM subtasks WHERE id = ?")
            .bind(&input.subtask_id)
            .execute(&self.pool)
            .await
            .map_err(Self::db_err)?;

        if result.rows_affected() == 0 {
            return Err(McpError::invalid_params(
                format!("Subtask not found: {}", input.subtask_id),
                None,
            ));
        }

        Self::json_result(&json!({"deleted": true, "subtask_id": input.subtask_id}))
    }

    #[tool(
        description = "List comments for a card in chronological order. Use this to review discussion history before making updates. Returns a JSON array of comments with id, card_id, author, content, and created_at."
    )]
    async fn kanban_get_comments(
        &self,
        Parameters(input): Parameters<GetCommentsInput>,
    ) -> Result<CallToolResult, McpError> {
        let comments: Vec<Comment> =
            sqlx::query_as("SELECT * FROM comments WHERE card_id = ? ORDER BY created_at ASC")
                .bind(&input.card_id)
                .fetch_all(&self.pool)
                .await
                .map_err(Self::db_err)?;

        Self::json_result(&comments)
    }

    #[tool(
        description = "Add a comment to a card. Use this to capture rationale, status, or implementation notes. Returns the created comment as JSON. Default author is AI Agent."
    )]
    async fn kanban_add_comment(
        &self,
        Parameters(input): Parameters<AddCommentInput>,
    ) -> Result<CallToolResult, McpError> {
        let _: Card = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(&input.card_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Self::db_err)?
            .ok_or_else(|| McpError::invalid_params(format!("Card not found: {}", input.card_id), None))?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let author = input.author.unwrap_or_else(|| "AI Agent".to_string());

        sqlx::query("INSERT INTO comments (id, card_id, author, content, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(&id)
            .bind(&input.card_id)
            .bind(&author)
            .bind(&input.content)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(Self::db_err)?;

        let comment: Comment = sqlx::query_as("SELECT * FROM comments WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .map_err(Self::db_err)?;

        Self::json_result(&comment)
    }

    #[tool(
        description = "Update comment content by id. Use this to revise existing notes while keeping history ordering. Returns the updated comment as JSON."
    )]
    async fn kanban_update_comment(
        &self,
        Parameters(input): Parameters<UpdateCommentInput>,
    ) -> Result<CallToolResult, McpError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query("UPDATE comments SET content = ?, created_at = ? WHERE id = ?")
            .bind(&input.content)
            .bind(&now)
            .bind(&input.comment_id)
            .execute(&self.pool)
            .await
            .map_err(Self::db_err)?;

        if result.rows_affected() == 0 {
            return Err(McpError::invalid_params(
                format!("Comment not found: {}", input.comment_id),
                None,
            ));
        }

        let comment: Comment = sqlx::query_as("SELECT * FROM comments WHERE id = ?")
            .bind(&input.comment_id)
            .fetch_one(&self.pool)
            .await
            .map_err(Self::db_err)?;

        Self::json_result(&comment)
    }

    #[tool(
        description = "Delete a comment by id. Use this to remove outdated or accidental notes. Returns a JSON object confirming deletion with the deleted comment id."
    )]
    async fn kanban_delete_comment(
        &self,
        Parameters(input): Parameters<DeleteCommentInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = sqlx::query("DELETE FROM comments WHERE id = ?")
            .bind(&input.comment_id)
            .execute(&self.pool)
            .await
            .map_err(Self::db_err)?;

        if result.rows_affected() == 0 {
            return Err(McpError::invalid_params(
                format!("Comment not found: {}", input.comment_id),
                None,
            ));
        }

        Self::json_result(&json!({"deleted": true, "comment_id": input.comment_id}))
    }
}

#[tool_handler]
impl ServerHandler for KanbanMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Kanban board management tools over direct SQLite access. Use board and card tools to inspect, create, update, and delete planning artifacts without HTTP round-trips.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
