use kanban_backend::config::Config;
use kanban_backend::infrastructure::db;
use kanban_backend::mcp::KanbanMcp;
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env().unwrap_or_default();
    let pool = db::init_db(&config.database_url).await?;

    let service = KanbanMcp::new(pool).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
