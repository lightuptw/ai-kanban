use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use kanban_backend::api::{create_router, AppState};
use kanban_backend::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,kanban_backend=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(
        "Starting Kanban Backend v{}...",
        env!("CARGO_PKG_VERSION")
    );

    let config = Config::from_env().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config from env, using defaults: {}", e);
        Config::default()
    });

    let (sse_tx, _rx) = broadcast::channel::<String>(100);
    let http_client = reqwest::Client::new();

    let state = AppState::new(None, sse_tx, http_client);
    let app = create_router(state, &config);

    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("Kanban Backend listening on http://{}", addr);
    tracing::info!("Health check: http://{}/health", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Kanban Backend shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down...");
        }
    }
}
