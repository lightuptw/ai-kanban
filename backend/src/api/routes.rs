use axum::http::HeaderValue;
use axum::routing::{get, patch, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::api::handlers;
use crate::api::state::AppState;
use crate::auth;
use crate::config::Config;

pub fn create_router(state: AppState, config: &Config) -> Router {
    let origins: Vec<HeaderValue> = config
        .cors_origin
        .split(',')
        .filter_map(|s| s.trim().parse::<HeaderValue>().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(origins)
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
        .route("/{id}/versions", get(handlers::cards::list_card_versions))
        .route(
            "/{id}/versions/{version_id}/restore",
            post(handlers::cards::restore_card_version),
        )
        .route("/{id}/move", patch(handlers::cards::move_card))
        .route("/{id}/diff", get(handlers::cards::get_card_diff))
        .route("/{id}/merge", post(handlers::cards::merge_card))
        .route("/{id}/create-pr", post(handlers::cards::create_card_pr))
        .route("/{id}/reject", post(handlers::cards::reject_card))
        .route("/{id}/generate-plan", post(handlers::cards::generate_plan))
        .route("/{id}/stop-ai", post(handlers::cards::stop_ai))
        .route("/{id}/resume-ai", post(handlers::cards::resume_ai))
        .route(
            "/{id}/questions/{question_id}/answer",
            post(handlers::questions::answer_question),
        )
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
        .route("/{id}/reorder", patch(handlers::boards::reorder_board))
        .route(
            "/{id}/settings",
            get(handlers::board_settings::get_board_settings)
                .put(handlers::board_settings::update_board_settings),
        )
        .route(
            "/{id}/settings/auto-detect",
            post(handlers::board_settings::auto_detect_board_settings),
        )
        .route(
            "/{id}/settings/clone-repo",
            post(handlers::board_settings::clone_repo),
        )
        .route(
            "/{id}/settings/auto-detect-status",
            get(handlers::board_settings::get_auto_detect_status),
        )
        .route(
            "/{id}/settings/auto-detect-logs",
            get(handlers::board_settings::get_auto_detect_logs),
        );

    let file_routes = Router::new().route(
        "/{id}",
        get(handlers::files::download_file).delete(handlers::files::delete_file),
    );

    let public_routes = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/health/live", get(handlers::liveness))
        .route(
            "/api/cards/{id}/questions",
            get(handlers::questions::get_questions).post(handlers::questions::create_question),
        )
        .route("/api/auth/register", post(auth::handlers::register))
        .route("/api/auth/login", post(auth::handlers::login))
        .route("/api/auth/refresh", post(auth::handlers::refresh));

    let protected_routes = Router::new()
        .route(
            "/api/pick-directory",
            post(handlers::picker::pick_directory),
        )
        .route("/api/pick-files", post(handlers::picker::pick_files))
        .route("/api/auth/me", get(auth::handlers::me))
        .route("/ws/events", get(handlers::ws::ws_events_handler))
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
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::middleware::auth_middleware,
        ));

    let api_routes = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let serve_dir = ServeDir::new(&config.frontend_dir).not_found_service(
        ServeDir::new(&config.frontend_dir).append_index_html_on_directories(true),
    );

    api_routes.fallback_service(serve_dir)
}
