use sqlx::SqlitePool;
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: Option<SqlitePool>,
    pub sse_tx: broadcast::Sender<String>,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new(
        db: Option<SqlitePool>,
        sse_tx: broadcast::Sender<String>,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            db,
            sse_tx,
            http_client,
        }
    }
}
