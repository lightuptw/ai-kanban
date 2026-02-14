use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteConnectOptions};
use sqlx::Row;
use std::str::FromStr;

pub async fn init_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let connect_options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    tracing::info!("Database initialized with WAL mode enabled");

    Ok(pool)
}

pub async fn verify_wal_mode(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let row = sqlx::query("PRAGMA journal_mode")
        .fetch_one(pool)
        .await?;
    
    Ok(row.get::<String, _>(0))
}
