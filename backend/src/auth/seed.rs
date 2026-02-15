use sqlx::SqlitePool;
use uuid::Uuid;

use super::password;

const DEFAULT_USERNAME: &str = "LightUp";
const DEFAULT_PASSWORD: &str = "Spark123";
const DEFAULT_NICKNAME: &str = "LightUp";

pub async fn seed_default_user(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE username = ?")
        .bind(DEFAULT_USERNAME)
        .fetch_optional(pool)
        .await?;

    if existing.is_some() {
        tracing::debug!("Default user '{}' already exists, skipping seed", DEFAULT_USERNAME);
        return Ok(());
    }

    let password_hash = password::hash_password(DEFAULT_PASSWORD)?;
    let user_id = Uuid::new_v4().to_string();
    let tenant_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, tenant_id, username, nickname, first_name, last_name, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(&tenant_id)
    .bind(DEFAULT_USERNAME)
    .bind(DEFAULT_NICKNAME)
    .bind("")
    .bind("")
    .bind("")
    .bind(&password_hash)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    tracing::info!("Default user '{}' created successfully", DEFAULT_USERNAME);
    Ok(())
}
