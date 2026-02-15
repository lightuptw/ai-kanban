use std::path::{Path, PathBuf};

use kanban_backend::mcp::KanbanMcp;
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_url =
        std::env::var("KANBAN_API_URL").unwrap_or_else(|_| "http://127.0.0.1:21547".to_string());
    let service_key = load_service_key();

    let service = KanbanMcp::new(api_url, service_key).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

fn load_service_key() -> Option<String> {
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

    tracing::warn!("Service key not found; MCP server will use JWT-only auth");
    None
}

fn read_service_key_file(path: &Path) -> Option<String> {
    let key = std::fs::read_to_string(path).ok()?;
    let trimmed = key.trim().to_string();
    if trimmed.is_empty() {
        return None;
    }
    tracing::info!("Loaded MCP service key from {}", path.display());
    Some(trimmed)
}

fn binary_parent_service_key_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let parent = exe.parent()?;
    Some(parent.join(".service-key"))
}
