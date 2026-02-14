use axum::http::HeaderValue;
use axum::routing::{get, patch, post};
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
        .route("/", post(handlers::cards::create_card))
        .route(
            "/{id}",
            get(handlers::cards::get_card)
                .patch(handlers::cards::update_card)
                .delete(handlers::cards::delete_card),
        )
        .route("/{id}/move", patch(handlers::cards::move_card))
        .route("/{id}/subtasks", post(handlers::subtasks::create_subtask))
        .route("/{id}/comments", post(handlers::comments::create_comment))
        .route(
            "/{id}/labels/{label_id}",
            post(handlers::labels::add_label).delete(handlers::labels::remove_label),
        );

    let subtask_routes = Router::new().route(
        "/{id}",
        patch(handlers::subtasks::update_subtask).delete(handlers::subtasks::delete_subtask),
    );

    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/health/live", get(handlers::liveness))
        .route("/api/events", get(handlers::sse::sse_handler))
        .route("/api/board", get(handlers::cards::get_board))
        .route("/api/labels", get(handlers::labels::list_labels))
        .nest("/api/cards", card_routes)
        .nest("/api/subtasks", subtask_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
