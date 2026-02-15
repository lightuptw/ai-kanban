use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub opencode_url: String,
    pub frontend_dir: String,
    pub cors_origin: String,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        Ok(Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(21547),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:kanban.db".into()),
            opencode_url: std::env::var("OPENCODE_URL")
                .unwrap_or_else(|_| "http://localhost:4096".into()),
            frontend_dir: std::env::var("FRONTEND_DIR")
                .unwrap_or_else(|_| "../frontend/dist".into()),
            cors_origin: std::env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:21548,http://127.0.0.1:21548".into()),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 21547,
            database_url: "sqlite:kanban.db".into(),
            opencode_url: "http://localhost:4096".into(),
            frontend_dir: "../frontend/dist".into(),
            cors_origin: "http://localhost:21548,http://127.0.0.1:21548".into(),
        }
    }
}
