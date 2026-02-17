use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;

use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::config::Config;
use crate::domain::KanbanError;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: Option<SqlitePool>,
    pub sse_tx: broadcast::Sender<String>,
    pub http_client: reqwest::Client,
    pub config: Arc<Config>,
    pub merge_locks: Arc<Mutex<HashSet<String>>>,
}

impl AppState {
    pub fn new(
        db: Option<SqlitePool>,
        sse_tx: broadcast::Sender<String>,
        http_client: reqwest::Client,
        config: Arc<Config>,
    ) -> Self {
        Self {
            db,
            sse_tx,
            http_client,
            config,
            merge_locks: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn require_db(&self) -> Result<&SqlitePool, KanbanError> {
        self.db
            .as_ref()
            .ok_or_else(|| KanbanError::Internal("Database not available".into()))
    }
}
