use std::path::{Path, PathBuf};

use rand::RngCore;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::password;

const DEFAULT_USERNAME: &str = "LightUp";
const DEFAULT_PASSWORD: &str = "Spark123";
const DEFAULT_NICKNAME: &str = "LightUp";
const SERVICE_USERNAME: &str = "__kanban_ai__";
const SERVICE_NICKNAME: &str = "AI Service";

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

pub async fn seed_service_account(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE username = ?")
        .bind(SERVICE_USERNAME)
        .fetch_optional(pool)
        .await?;

    if existing.is_some() {
        let key_file_path = service_key_file_path();
        if !key_file_path.exists() {
            let service_key: Option<(Vec<u8>,)> =
                sqlx::query_as("SELECT value FROM app_secrets WHERE key = ?")
                    .bind("service_api_key")
                    .fetch_optional(pool)
                    .await?;

            if let Some((service_key,)) = service_key {
                let key = String::from_utf8(service_key)?;
                write_service_key_file(&key_file_path, &key)?;
                tracing::info!("Service API key file written to {}", key_file_path.display());
            } else {
                tracing::warn!("Service account exists but service_api_key is missing in app_secrets");
            }
        }

        tracing::debug!(
            "Service account '{}' already exists, skipping seed",
            SERVICE_USERNAME
        );
        return Ok(());
    }

    let mut password_bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut password_bytes);
    let password: String = password_bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect();

    let password_hash = password::hash_password(&password)?;
    let user_id = Uuid::new_v4().to_string();
    let tenant_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, tenant_id, username, nickname, first_name, last_name, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(&tenant_id)
    .bind(SERVICE_USERNAME)
    .bind(SERVICE_NICKNAME)
    .bind("")
    .bind("")
    .bind("")
    .bind(&password_hash)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let mut service_key_bytes = [0_u8; 64];
    rand::thread_rng().fill_bytes(&mut service_key_bytes);
    let service_key: String = service_key_bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect();

    sqlx::query("INSERT OR REPLACE INTO app_secrets (key, value, created_at) VALUES (?, ?, ?)")
        .bind("service_api_key")
        .bind(service_key.as_bytes())
        .bind(&now)
        .execute(pool)
        .await?;

    sqlx::query("INSERT OR REPLACE INTO app_secrets (key, value, created_at) VALUES (?, ?, ?)")
        .bind("service_user_id")
        .bind(user_id.as_bytes())
        .bind(&now)
        .execute(pool)
        .await?;

    let key_file_path = service_key_file_path();
    write_service_key_file(&key_file_path, &service_key)?;
    tracing::info!("Service API key file written to {}", key_file_path.display());

    tracing::info!("Service account '{}' created successfully", SERVICE_USERNAME);
    Ok(())
}

fn write_service_key_file(path: &Path, key: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, key.as_bytes())?;
    Ok(())
}

fn service_key_file_path() -> PathBuf {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:kanban.db".to_string());
    let db_path = db_url
        .strip_prefix("sqlite:")
        .unwrap_or(db_url.as_str());
    let parent = Path::new(db_path)
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    parent.join(".service-key")
}
