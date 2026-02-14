use axum::http::HeaderValue;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::api::handlers;
use crate::api::state::AppState;
use crate::config::Config;

pub fn create_router(state: AppState, config: &Config) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            config
                .cors_origin
                .parse::<HeaderValue>()
                .expect("Invalid CORS origin"),
        )
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let card_routes = Router::new()
        .route("/", get(handlers::cards::list_cards))
        .route("/", post(handlers::cards::create_card))
        .route("/{id}", get(handlers::cards::get_card))
        .route("/{id}", patch(handlers::cards::update_card))
        .route("/{id}", delete(handlers::cards::delete_card))
        .route("/{id}/move", post(handlers::cards::move_card));

    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/health/live", get(handlers::liveness))
        .route("/api/events", get(handlers::sse::sse_handler))
        .nest("/api/cards", card_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
