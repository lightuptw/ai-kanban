use sqlx::sqlite::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlitePool::connect("sqlite:kanban.db").await?;
    
    let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .fetch_all(&pool)
        .await?;
    
    println!("Tables: {}", tables.join(", "));
    
    let label_count: i64 = sqlx::query_scalar("SELECT count(*) FROM labels")
        .fetch_one(&pool)
        .await?;
    
    println!("Label count: {}", label_count);
    
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await?;
    
    println!("Journal mode: {}", journal_mode);
    
    Ok(())
}
