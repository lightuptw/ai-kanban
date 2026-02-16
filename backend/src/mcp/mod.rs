use std::path::{Path, PathBuf};

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

#[derive(Clone)]
pub struct KanbanMcp {
    client: reqwest::Client,
    base_url: String,
    service_key: Option<String>,
    tool_router: ToolRouter<Self>,
}

pub trait IntoKanbanApiUrl {
    fn into_kanban_api_url(self) -> String;
}

impl IntoKanbanApiUrl for String {
    fn into_kanban_api_url(self) -> String {
        self
    }
}

impl IntoKanbanApiUrl for &str {
    fn into_kanban_api_url(self) -> String {
        self.to_string()
    }
}

impl IntoKanbanApiUrl for sqlx::SqlitePool {
    fn into_kanban_api_url(self) -> String {
        std::env::var("KANBAN_API_URL").unwrap_or_else(|_| "http://127.0.0.1:21547".to_string())
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct BoardInput {
    /// Action: "list" (default), "create", or "delete"
    #[serde(default = "default_list")]
    action: String,
    /// Board name (required for "create")
    name: Option<String>,
    /// Board ID (required for "delete")
    board_id: Option<String>,
}

fn default_list() -> String {
    "list".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct GetBoardCardsInput {
    board_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct GetCardInput {
    card_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct CreateCardInput {
    title: String,
    description: Option<String>,
    stage: Option<String>,
    priority: Option<String>,
    board_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct UpdateCardInput {
    card_id: String,
    title: Option<String>,
    description: Option<String>,
    stage: Option<String>,
    priority: Option<String>,
    working_directory: Option<String>,
    linked_documents: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct DeleteCardInput {
    card_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct CreateSubtaskInput {
    card_id: String,
    title: String,
    phase: Option<String>,
    phase_order: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct UpdateSubtaskInput {
    subtask_id: String,
    title: Option<String>,
    completed: Option<bool>,
    phase: Option<String>,
    phase_order: Option<i64>,
    position: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct DeleteSubtaskInput {
    subtask_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct CommentInput {
    card_id: String,
    /// Action to perform: "add" (default), "list", "update", or "delete"
    #[serde(default = "default_add")]
    action: String,
    /// Comment content (required for "add" and "update")
    content: Option<String>,
    /// Comment author (for "add", defaults to "AI Agent")
    author: Option<String>,
    /// Comment ID (required for "update" and "delete")
    comment_id: Option<String>,
}

fn default_add() -> String {
    "add".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct BoardSettingsInput {
    /// Board ID (required)
    board_id: String,
    /// Action: "get" (default) or "update"
    #[serde(default = "default_get")]
    action: String,
    /// Fields below are only used for "update" action:
    codebase_path: Option<String>,
    github_repo: Option<String>,
    context_markdown: Option<String>,
    document_links: Option<String>,
    variables: Option<String>,
    tech_stack: Option<String>,
    communication_patterns: Option<String>,
    environments: Option<String>,
    code_conventions: Option<String>,
    testing_requirements: Option<String>,
    api_conventions: Option<String>,
    infrastructure: Option<String>,
}

fn default_get() -> String {
    "get".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct AskQuestionInput {
    /// The card ID this question belongs to
    card_id: String,
    /// The question text to ask the user
    question: String,
    /// Question type: "select", "multi_select", or "text"
    #[serde(default = "default_select")]
    question_type: String,
    /// JSON array of options: [{"label": "...", "description": "..."}]
    #[serde(default = "default_empty_array")]
    options: String,
    /// Allow multiple selections (for multi_select type)
    #[serde(default)]
    multiple: bool,
}

fn default_select() -> String {
    "select".to_string()
}

fn default_empty_array() -> String {
    "[]".to_string()
}

#[tool_router]
impl KanbanMcp {
    pub fn new<T: IntoKanbanApiUrl>(base_url: T, service_key: Option<String>) -> Self {
        let service_key = resolve_service_key(service_key);
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into_kanban_api_url(),
            service_key,
            tool_router: Self::tool_router(),
        }
    }

    fn api_err(msg: String) -> McpError {
        McpError::internal_error(msg, None)
    }

    fn json_result(value: &serde_json::Value) -> Result<CallToolResult, McpError> {
        let text = serde_json::to_string_pretty(value)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    fn with_service_key(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.service_key {
            req.header("X-Service-Key", key)
        } else {
            req
        }
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value, McpError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .with_service_key(self.client.get(&url))
            .send()
            .await
            .map_err(|e| Self::api_err(format!("HTTP GET {}: {}", path, e)))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::internal_error(
                format!("API error {}: {}", status, body),
                None,
            ));
        }
        resp.json()
            .await
            .map_err(|e| Self::api_err(format!("JSON decode: {}", e)))
    }

    async fn post(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .with_service_key(self.client.post(&url))
            .json(body)
            .send()
            .await
            .map_err(|e| Self::api_err(format!("HTTP POST {}: {}", path, e)))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(McpError::internal_error(
                format!("API error {}: {}", status, body_text),
                None,
            ));
        }
        resp.json()
            .await
            .map_err(|e| Self::api_err(format!("JSON decode: {}", e)))
    }

    async fn patch(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .with_service_key(self.client.patch(&url))
            .json(body)
            .send()
            .await
            .map_err(|e| Self::api_err(format!("HTTP PATCH {}: {}", path, e)))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(McpError::internal_error(
                format!("API error {}: {}", status, body_text),
                None,
            ));
        }
        resp.json()
            .await
            .map_err(|e| Self::api_err(format!("JSON decode: {}", e)))
    }

    async fn put(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .with_service_key(self.client.put(&url))
            .json(body)
            .send()
            .await
            .map_err(|e| Self::api_err(format!("HTTP PUT {}: {}", path, e)))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(McpError::internal_error(
                format!("API error {}: {}", status, body_text),
                None,
            ));
        }
        resp.json()
            .await
            .map_err(|e| Self::api_err(format!("JSON decode: {}", e)))
    }

    async fn delete(&self, path: &str) -> Result<serde_json::Value, McpError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .with_service_key(self.client.delete(&url))
            .send()
            .await
            .map_err(|e| Self::api_err(format!("HTTP DELETE {}: {}", path, e)))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::internal_error(
                format!("API error {}: {}", status, body),
                None,
            ));
        }
        let text = resp.text().await.unwrap_or_default();
        if text.is_empty() {
            Ok(json!({"deleted": true}))
        } else {
            serde_json::from_str(&text).map_err(|e| Self::api_err(format!("JSON decode: {}", e)))
        }
    }

    #[tool(
        description = "Manage boards. Actions: \"list\" (default, no params needed), \"create\" (requires name), \"delete\" (requires board_id). Returns board JSON."
    )]
    async fn kanban_board(
        &self,
        Parameters(input): Parameters<BoardInput>,
    ) -> Result<CallToolResult, McpError> {
        match input.action.as_str() {
            "list" => {
                let data = self.get("/api/boards").await?;
                Self::json_result(&data)
            }
            "create" => {
                let name = input.name.ok_or_else(|| {
                    McpError::internal_error("name is required for action 'create'", None)
                })?;
                let data = self.post("/api/boards", &json!({"name": name})).await?;
                Self::json_result(&data)
            }
            "delete" => {
                let board_id = input.board_id.ok_or_else(|| {
                    McpError::internal_error("board_id is required for action 'delete'", None)
                })?;
                let data = self.delete(&format!("/api/boards/{}", board_id)).await?;
                Self::json_result(&data)
            }
            other => Err(McpError::internal_error(
                format!("Unknown action '{}'. Valid: list, create, delete", other),
                None,
            )),
        }
    }

    #[tool(
        description = "Fetch board cards grouped by workflow stage. Use this as the primary board overview call before planning or acting on tasks. Returns a JSON object with keys backlog, plan, todo, in_progress, review, and done, each containing card summary arrays."
    )]
    async fn kanban_get_board_cards(
        &self,
        Parameters(input): Parameters<GetBoardCardsInput>,
    ) -> Result<CallToolResult, McpError> {
        let board_id = input.board_id.unwrap_or_else(|| "default".to_string());
        let data = self
            .get(&format!("/api/board?board_id={}", board_id))
            .await?;
        Self::json_result(&data)
    }

    #[tool(
        description = "Get full details for one card, including subtasks, comments, and labels. Use this when you need complete context before editing or executing work. Returns a JSON object with card, subtasks, comments, and labels."
    )]
    async fn kanban_get_card(
        &self,
        Parameters(input): Parameters<GetCardInput>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.get(&format!("/api/cards/{}", input.card_id)).await?;
        Self::json_result(&data)
    }

    #[tool(
        description = "Create a new card on a board. Use this when starting a task, bug, or feature. Returns the created card as JSON. Defaults: stage=backlog, priority=medium, board_id=default, working_directory=."
    )]
    async fn kanban_create_card(
        &self,
        Parameters(input): Parameters<CreateCardInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut body = serde_json::Map::new();
        body.insert("title".into(), json!(input.title));
        if let Some(v) = input.description {
            body.insert("description".into(), json!(v));
        }
        if let Some(v) = input.stage {
            body.insert("stage".into(), json!(v));
        }
        if let Some(v) = input.priority {
            body.insert("priority".into(), json!(v));
        }
        if let Some(v) = input.board_id {
            body.insert("board_id".into(), json!(v));
        }
        let data = self.post("/api/cards", &serde_json::Value::Object(body)).await?;
        Self::json_result(&data)
    }

    #[tool(
        description = "Update card fields by id. Use this when card details, status, working directory, or linked_documents change. Returns the updated card as JSON. Stage options: backlog, plan, todo, in_progress, review, done."
    )]
    async fn kanban_update_card(
        &self,
        Parameters(input): Parameters<UpdateCardInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut body = serde_json::Map::new();
        if let Some(v) = input.title {
            body.insert("title".into(), json!(v));
        }
        if let Some(v) = input.description {
            body.insert("description".into(), json!(v));
        }
        if let Some(v) = input.stage {
            body.insert("stage".into(), json!(v));
        }
        if let Some(v) = input.priority {
            body.insert("priority".into(), json!(v));
        }
        if let Some(v) = input.working_directory {
            body.insert("working_directory".into(), json!(v));
        }
        if let Some(v) = input.linked_documents {
            body.insert("linked_documents".into(), json!(v));
        }
        let data = self
            .patch(
                &format!("/api/cards/{}", input.card_id),
                &serde_json::Value::Object(body),
            )
            .await?;
        Self::json_result(&data)
    }

    #[tool(
        description = "Delete a card by id. Use this when removing obsolete or duplicate work items. Returns a JSON object confirming deletion with the deleted card id."
    )]
    async fn kanban_delete_card(
        &self,
        Parameters(input): Parameters<DeleteCardInput>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.delete(&format!("/api/cards/{}", input.card_id)).await?;
        Self::json_result(&data)
    }

    #[tool(
        description = "Create a subtask on a card. Use this to break work into actionable checklist items. Returns the created subtask as JSON. Defaults: phase=Phase 1, phase_order=1, and position is appended within the card."
    )]
    async fn kanban_create_subtask(
        &self,
        Parameters(input): Parameters<CreateSubtaskInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut body = json!({"title": input.title});
        if let Some(phase) = input.phase {
            body["phase"] = json!(phase);
        }
        if let Some(phase_order) = input.phase_order {
            body["phase_order"] = json!(phase_order);
        }
        let data = self
            .post(&format!("/api/cards/{}/subtasks", input.card_id), &body)
            .await?;
        Self::json_result(&data)
    }

    #[tool(
        description = "Update a subtask by id. Use this to rename items, check them off, or reorder/phase them. Returns the updated subtask as JSON."
    )]
    async fn kanban_update_subtask(
        &self,
        Parameters(input): Parameters<UpdateSubtaskInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut body = serde_json::Map::new();
        if let Some(v) = input.title {
            body.insert("title".into(), json!(v));
        }
        if let Some(v) = input.completed {
            body.insert("completed".into(), json!(v));
        }
        if let Some(v) = input.phase {
            body.insert("phase".into(), json!(v));
        }
        if let Some(v) = input.phase_order {
            body.insert("phase_order".into(), json!(v));
        }
        if let Some(v) = input.position {
            body.insert("position".into(), json!(v));
        }
        let data = self
            .patch(
                &format!("/api/subtasks/{}", input.subtask_id),
                &serde_json::Value::Object(body),
            )
            .await?;
        Self::json_result(&data)
    }

    #[tool(
        description = "Delete a subtask by id. Use this when a checklist item is no longer needed. Returns a JSON object confirming deletion with the deleted subtask id."
    )]
    async fn kanban_delete_subtask(
        &self,
        Parameters(input): Parameters<DeleteSubtaskInput>,
    ) -> Result<CallToolResult, McpError> {
        let data = self
            .delete(&format!("/api/subtasks/{}", input.subtask_id))
            .await?;
        Self::json_result(&data)
    }

    #[tool(
        description = "Manage comments on a card. Supports 4 actions via the 'action' parameter: \"list\" (list all comments, only card_id required), \"add\" (default, requires content, optional author defaults to 'AI Agent'), \"update\" (requires comment_id and content), \"delete\" (requires comment_id). Returns the comment(s) as JSON."
    )]
    async fn kanban_comment(
        &self,
        Parameters(input): Parameters<CommentInput>,
    ) -> Result<CallToolResult, McpError> {
        match input.action.as_str() {
            "list" => {
                let data = self
                    .get(&format!("/api/cards/{}/comments", input.card_id))
                    .await?;
                Self::json_result(&data)
            }
            "add" => {
                let content = input.content.ok_or_else(|| {
                    McpError::internal_error(
                        "content is required for action 'add'".to_string(),
                        None,
                    )
                })?;
                let body = json!({
                    "author": input.author.unwrap_or_else(|| "AI Agent".to_string()),
                    "content": content
                });
                let data = self
                    .post(&format!("/api/cards/{}/comments", input.card_id), &body)
                    .await?;
                Self::json_result(&data)
            }
            "update" => {
                let comment_id = input.comment_id.ok_or_else(|| {
                    McpError::internal_error(
                        "comment_id is required for action 'update'".to_string(),
                        None,
                    )
                })?;
                let content = input.content.ok_or_else(|| {
                    McpError::internal_error(
                        "content is required for action 'update'".to_string(),
                        None,
                    )
                })?;
                let body = json!({"content": content});
                let data = self
                    .patch(&format!("/api/comments/{}", comment_id), &body)
                    .await?;
                Self::json_result(&data)
            }
            "delete" => {
                let comment_id = input.comment_id.ok_or_else(|| {
                    McpError::internal_error(
                        "comment_id is required for action 'delete'".to_string(),
                        None,
                    )
                })?;
                let data = self
                    .delete(&format!("/api/comments/{}", comment_id))
                    .await?;
                Self::json_result(&data)
            }
            other => Err(McpError::internal_error(
                format!(
                    "Unknown action '{}'. Valid actions: list, add, update, delete",
                    other
                ),
                None,
            )),
        }
    }

    #[tool(
        description = "Manage board-level settings. Actions: \"get\" (default, returns codebase path, AI context, tech stack, conventions, environment details), \"update\" (set any settings fields). Requires board_id."
    )]
    async fn kanban_board_settings(
        &self,
        Parameters(input): Parameters<BoardSettingsInput>,
    ) -> Result<CallToolResult, McpError> {
        let path = format!("/api/boards/{}/settings", input.board_id);
        match input.action.as_str() {
            "get" => {
                let data = self.get(&path).await?;
                Self::json_result(&data)
            }
            "update" => {
                let mut body = serde_json::Map::new();
                if let Some(v) = input.codebase_path {
                    body.insert("codebase_path".into(), json!(v));
                }
                if let Some(v) = input.github_repo {
                    body.insert("github_repo".into(), json!(v));
                }
                if let Some(v) = input.context_markdown {
                    body.insert("context_markdown".into(), json!(v));
                }
                if let Some(v) = input.document_links {
                    body.insert("document_links".into(), json!(v));
                }
                if let Some(v) = input.variables {
                    body.insert("variables".into(), json!(v));
                }
                if let Some(v) = input.tech_stack {
                    body.insert("tech_stack".into(), json!(v));
                }
                if let Some(v) = input.communication_patterns {
                    body.insert("communication_patterns".into(), json!(v));
                }
                if let Some(v) = input.environments {
                    body.insert("environments".into(), json!(v));
                }
                if let Some(v) = input.code_conventions {
                    body.insert("code_conventions".into(), json!(v));
                }
                if let Some(v) = input.testing_requirements {
                    body.insert("testing_requirements".into(), json!(v));
                }
                if let Some(v) = input.api_conventions {
                    body.insert("api_conventions".into(), json!(v));
                }
                if let Some(v) = input.infrastructure {
                    body.insert("infrastructure".into(), json!(v));
                }
                let data = self.put(&path, &serde_json::Value::Object(body)).await?;
                Self::json_result(&data)
            }
            other => Err(McpError::internal_error(
                format!("Unknown action '{}'. Valid: get, update", other),
                None,
            )),
        }
    }

    #[tool(
        description = "Ask the user a question and wait for their answer. Use this when you need user input before proceeding. For select/multi_select types, provide options as a JSON array of objects with 'label' and 'description' fields. The tool will block until the user responds. Returns the user's answer."
    )]
    async fn kanban_ask_question(
        &self,
        Parameters(input): Parameters<AskQuestionInput>,
    ) -> Result<CallToolResult, McpError> {
        let body = json!({
            "question": input.question,
            "question_type": input.question_type,
            "options": input.options,
            "multiple": input.multiple,
        });

        let data = self
            .post(&format!("/api/cards/{}/questions", input.card_id), &body)
            .await?;

        let question_id = data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let max_attempts = 600;
        for _ in 0..max_attempts {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            let result = self
                .get(&format!("/api/cards/{}/questions", input.card_id))
                .await?;

            if let Some(questions) = result.as_array() {
                if let Some(q) = questions
                    .iter()
                    .find(|q| q.get("id").and_then(|v| v.as_str()) == Some(question_id.as_str()))
                {
                    if let Some(answer) = q.get("answer") {
                        if !answer.is_null() {
                            return Ok(CallToolResult::success(vec![Content::text(format!(
                                "User answered: {}",
                                answer
                            ))]));
                        }
                    }
                }
            }
        }

        Err(McpError::internal_error(
            "Question timed out after 30 minutes".to_string(),
            None,
        ))
    }
}

#[tool_handler]
impl ServerHandler for KanbanMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Kanban board management tools. Proxies to the kanban REST API for all operations."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

fn resolve_service_key(explicit_key: Option<String>) -> Option<String> {
    if let Some(key) = explicit_key {
        return Some(key);
    }

    if let Ok(key) = std::env::var("KANBAN_SERVICE_KEY") {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }

    if let Some(key) = read_service_key_file(Path::new(".service-key")) {
        return Some(key);
    }

    if let Some(path) = binary_parent_service_key_path() {
        if let Some(key) = read_service_key_file(&path) {
            return Some(key);
        }
    }

    tracing::warn!(
        "No service API key found in KANBAN_SERVICE_KEY or .service-key; MCP requests will use JWT-only auth"
    );
    None
}

fn read_service_key_file(path: &Path) -> Option<String> {
    let key = std::fs::read_to_string(path).ok()?;
    let trimmed = key.trim().to_string();
    if trimmed.is_empty() {
        return None;
    }
    tracing::info!("Loaded MCP service API key from {}", path.display());
    Some(trimmed)
}

fn binary_parent_service_key_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let parent = exe.parent()?;
    Some(parent.join(".service-key"))
}
