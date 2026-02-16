mod common;

use axum::http::StatusCode;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_health_check() {
    let (pool, _token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();
    
    let config = Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
        cookie_secure: false,
    });

    let state = kanban_backend::api::state::AppState {
        db: Some(pool),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);
    
    let (status, body) = common::make_request(app, "GET", "/health", None, None).await;
    
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"status\":\"ok\""));
}

#[tokio::test]
async fn test_create_and_get_card() {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();
    
    let config = Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
        cookie_secure: false,
    });

    let state = kanban_backend::api::state::AppState {
        db: Some(pool),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);

    let create_body = json!({
        "title": "Test Card",
        "description": "Test description",
        "priority": "high"
    })
    .to_string();

    let (status, body) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(create_body),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    
    let card: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(card["title"], "Test Card");
    assert_eq!(card["stage"], "backlog");
    assert_eq!(card["priority"], "high");
    
    let card_id = card["id"].as_str().unwrap();

    let (status, body) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let fetched_card: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(fetched_card["id"], card_id);
    assert_eq!(fetched_card["title"], "Test Card");
}

#[tokio::test]
async fn test_move_card_between_stages() {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();
    
    let config = Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
        cookie_secure: false,
    });

    let state = kanban_backend::api::state::AppState {
        db: Some(pool),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);

    let create_body = json!({
        "title": "Move Test Card",
        "description": "Testing stage transitions"
    })
    .to_string();

    let (_, body) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(create_body),
        Some(&token),
    )
    .await;

    let card: serde_json::Value = serde_json::from_str(&body).unwrap();
    let card_id = card["id"].as_str().unwrap();

    let move_body = json!({
        "stage": "plan",
        "position": 1000
    })
    .to_string();

    let (status, body) = common::make_request(
        app,
        "PATCH",
        &format!("/api/cards/{}/move", card_id),
        Some(move_body),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let moved_card: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(moved_card["stage"], "plan");
    assert_eq!(moved_card["position"], 1000);
}

#[tokio::test]
async fn test_invalid_stage_transition() {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();
    
    let config = Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
        cookie_secure: false,
    });

    let state = kanban_backend::api::state::AppState {
        db: Some(pool),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);

    let create_body = json!({
        "title": "FSM Test Card"
    })
    .to_string();

    let (_, body) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(create_body),
        Some(&token),
    )
    .await;

    let card: serde_json::Value = serde_json::from_str(&body).unwrap();
    let card_id = card["id"].as_str().unwrap();

    let move_body = json!({
        "stage": "done",
        "position": 1000
    })
    .to_string();

    let (status, _) = common::make_request(
        app,
        "PATCH",
        &format!("/api/cards/{}/move", card_id),
        Some(move_body),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_delete_card() {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();
    
    let config = Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
        cookie_secure: false,
    });

    let state = kanban_backend::api::state::AppState {
        db: Some(pool),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);

    let create_body = json!({
        "title": "Delete Test Card"
    })
    .to_string();

    let (_, body) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(create_body),
        Some(&token),
    )
    .await;

    let card: serde_json::Value = serde_json::from_str(&body).unwrap();
    let card_id = card["id"].as_str().unwrap();

    let (status, _) = common::make_request(
        app.clone(),
        "DELETE",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_board() {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();
    
    let config = Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
        cookie_secure: false,
    });

    let state = kanban_backend::api::state::AppState {
        db: Some(pool),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);

    let (status, body) = common::make_request(app, "GET", "/api/board", None, Some(&token)).await;

    assert_eq!(status, StatusCode::OK);
    let board: serde_json::Value = serde_json::from_str(&body).unwrap();
    
    assert!(board["backlog"].is_array());
    assert!(board["plan"].is_array());
    assert!(board["todo"].is_array());
    assert!(board["in_progress"].is_array());
    assert!(board["review"].is_array());
    assert!(board["done"].is_array());
}
