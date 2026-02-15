use kanban_backend::mcp::KanbanMcp;
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_url =
        std::env::var("KANBAN_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let service = KanbanMcp::new(api_url).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
