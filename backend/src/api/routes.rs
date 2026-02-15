use axum::http::HeaderValue;
use axum::routing::{get, patch, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
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
        .route("/{id}/logs", get(handlers::cards::get_card_logs))
        .route("/{id}/move", patch(handlers::cards::move_card))
        .route("/{id}/subtasks", post(handlers::subtasks::create_subtask))
        .route(
            "/{id}/comments",
            get(handlers::comments::get_comments).post(handlers::comments::create_comment),
        )
        .route(
            "/{id}/labels/{label_id}",
            post(handlers::labels::add_label).delete(handlers::labels::remove_label),
        )
        .route(
            "/{id}/files",
            post(handlers::files::upload_files).get(handlers::files::list_card_files),
        );

    let comment_routes = Router::new().route(
        "/{id}",
        patch(handlers::comments::update_comment).delete(handlers::comments::delete_comment),
    );

    let subtask_routes = Router::new().route(
        "/{id}",
        patch(handlers::subtasks::update_subtask).delete(handlers::subtasks::delete_subtask),
    );

    let board_routes = Router::new()
        .route(
            "/",
            get(handlers::boards::list_boards).post(handlers::boards::create_board),
        )
        .route(
            "/{id}",
            patch(handlers::boards::update_board).delete(handlers::boards::delete_board),
        )
        .route("/{id}/reorder", patch(handlers::boards::reorder_board));

    let file_routes = Router::new().route(
        "/{id}",
        get(handlers::files::download_file).delete(handlers::files::delete_file),
    );

    let api_routes = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/health/live", get(handlers::liveness))
        .route(
            "/api/pick-directory",
            post(handlers::picker::pick_directory),
        )
        .route("/api/pick-files", post(handlers::picker::pick_files))
        .route("/api/events", get(handlers::sse::sse_handler))
        .route("/ws/logs/{card_id}", get(handlers::ws::ws_logs_handler))
        .route("/api/board", get(handlers::cards::get_board))
        .route("/api/labels", get(handlers::labels::list_labels))
        .nest("/api/boards", board_routes)
        .nest("/api/cards", card_routes)
        .nest("/api/subtasks", subtask_routes)
        .nest("/api/comments", comment_routes)
        .nest("/api/files", file_routes)
        .route(
            "/api/settings/{key}",
            get(handlers::settings::get_setting).put(handlers::settings::set_setting),
        )
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let serve_dir = ServeDir::new(&config.frontend_dir).not_found_service(
        ServeDir::new(&config.frontend_dir).append_index_html_on_directories(true),
    );

    api_routes.fallback_service(serve_dir)
}
