mod common;

use axum::http::StatusCode;
use serde_json::json;
use std::sync::Arc;

fn test_config() -> Arc<kanban_backend::config::Config> {
    Arc::new(kanban_backend::config::Config {
        port: 21547,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:21548".to_string(),
    })
}

async fn test_app() -> (axum::Router, String) {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();
    let config = test_config();

    let state = kanban_backend::api::state::AppState {
        db: Some(pool),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);
    (app, token)
}

async fn test_app_with_pool() -> (axum::Router, String, sqlx::SqlitePool) {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();
    let config = test_config();

    let state = kanban_backend::api::state::AppState {
        db: Some(pool.clone()),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);
    (app, token, pool)
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_check() {
    let (app, _) = test_app().await;
    let (status, body) = common::make_request(app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"status\":\"ok\""));
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_auth_register_and_login() {
    let (app, _) = test_app().await;

    let reg_body = json!({
        "username": "newuser",
        "password": "SecurePass123",
        "nickname": "New User"
    })
    .to_string();

    let (status, body) = common::make_request(
        app.clone(),
        "POST",
        "/api/auth/register",
        Some(reg_body),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Register failed: {}", body);
    let auth: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(auth["token"].is_string());
    assert!(auth["refresh_token"].is_string());
    assert_eq!(auth["user"]["username"], "newuser");

    let login_body = json!({
        "username": "newuser",
        "password": "SecurePass123"
    })
    .to_string();

    let (status, body) = common::make_request(
        app.clone(),
        "POST",
        "/api/auth/login",
        Some(login_body),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Login failed: {}", body);
    let auth: serde_json::Value = serde_json::from_str(&body).unwrap();
    let token = auth["token"].as_str().unwrap();

    let (status, body) =
        common::make_request(app, "GET", "/api/auth/me", None, Some(token)).await;
    assert_eq!(status, StatusCode::OK, "Me failed: {}", body);
    let me: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(me["username"], "newuser");
}

#[tokio::test]
async fn test_auth_login_wrong_password() {
    let (app, _) = test_app().await;

    let login_body = json!({
        "username": "test_user",
        "password": "WrongPassword"
    })
    .to_string();

    let (status, _) = common::make_request(
        app,
        "POST",
        "/api/auth/login",
        Some(login_body),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_protected_endpoint_without_token() {
    let (app, _) = test_app().await;
    let (status, _) = common::make_request(app, "GET", "/api/board", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_refresh_token() {
    let (app, _) = test_app().await;

    let reg_body = json!({
        "username": "refreshuser",
        "password": "SecurePass123",
        "nickname": "Refresh User"
    })
    .to_string();

    let (_, body) = common::make_request(
        app.clone(),
        "POST",
        "/api/auth/register",
        Some(reg_body),
        None,
    )
    .await;
    let auth: serde_json::Value = serde_json::from_str(&body).unwrap();
    let refresh_token = auth["refresh_token"].as_str().unwrap();

    let refresh_body = json!({ "refresh_token": refresh_token }).to_string();
    let (status, body) = common::make_request(
        app,
        "POST",
        "/api/auth/refresh",
        Some(refresh_body),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Refresh failed: {}", body);
    let new_auth: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(new_auth["token"].is_string());
}

// ---------------------------------------------------------------------------
// Cards — CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_and_get_card() {
    let (app, token) = test_app().await;

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
async fn test_update_card() {
    let (app, token) = test_app().await;

    let create_body = json!({ "title": "Update Me" }).to_string();
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

    let update_body = json!({
        "title": "Updated Title",
        "description": "New description",
        "priority": "critical",
        "working_directory": "/tmp/test",
        "linked_documents": "[\"/docs/spec.md\"]"
    })
    .to_string();

    let (status, body) = common::make_request(
        app.clone(),
        "PATCH",
        &format!("/api/cards/{}", card_id),
        Some(update_body),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "Update card failed: {}", body);
    let updated: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(updated["title"], "Updated Title");
    assert_eq!(updated["description"], "New description");
    assert_eq!(updated["priority"], "critical");
    assert_eq!(updated["working_directory"], "/tmp/test");
}

#[tokio::test]
async fn test_move_card_between_stages() {
    let (app, token) = test_app().await;

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
    let (app, token) = test_app().await;

    let create_body = json!({ "title": "FSM Test Card" }).to_string();
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

    let move_body = json!({ "stage": "done", "position": 1000 }).to_string();
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
    let (app, token) = test_app().await;

    let create_body = json!({ "title": "Delete Test Card" }).to_string();
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
async fn test_get_card_not_found() {
    let (app, token) = test_app().await;
    let (status, _) = common::make_request(
        app,
        "GET",
        "/api/cards/nonexistent-id",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Board view
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_board() {
    let (app, token) = test_app().await;

    let (status, body) =
        common::make_request(app, "GET", "/api/board", None, Some(&token)).await;

    assert_eq!(status, StatusCode::OK);
    let board: serde_json::Value = serde_json::from_str(&body).unwrap();

    assert!(board["backlog"].is_array());
    assert!(board["plan"].is_array());
    assert!(board["todo"].is_array());
    assert!(board["in_progress"].is_array());
    assert!(board["review"].is_array());
    assert!(board["done"].is_array());
}

#[tokio::test]
async fn test_get_board_cards_appear_in_correct_stage() {
    let (app, token) = test_app().await;

    let create_body = json!({ "title": "Board View Card" }).to_string();
    common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(create_body),
        Some(&token),
    )
    .await;

    let (_, body) =
        common::make_request(app, "GET", "/api/board", None, Some(&token)).await;
    let board: serde_json::Value = serde_json::from_str(&body).unwrap();
    let backlog = board["backlog"].as_array().unwrap();
    assert!(
        backlog.iter().any(|c| c["title"] == "Board View Card"),
        "New card should appear in backlog"
    );
}

// ---------------------------------------------------------------------------
// Boards — CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_list_boards() {
    let (app, token) = test_app().await;

    let (status, body) =
        common::make_request(app, "GET", "/api/boards", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let boards: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(boards.is_array());
    let arr = boards.as_array().unwrap();
    assert!(
        arr.iter().any(|b| b["id"] == "default"),
        "Default board should exist"
    );
}

#[tokio::test]
async fn test_create_board() {
    let (app, token) = test_app().await;

    let body = json!({ "name": "Sprint 42" }).to_string();
    let (status, resp) =
        common::make_request(app, "POST", "/api/boards", Some(body), Some(&token)).await;
    assert_eq!(status, StatusCode::CREATED, "Create board failed: {}", resp);
    let board: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(board["name"], "Sprint 42");
    assert!(board["id"].is_string());
    assert!(board["position"].is_number());
}

#[tokio::test]
async fn test_update_board() {
    let (app, token) = test_app().await;

    let body = json!({ "name": "Old Name" }).to_string();
    let (_, resp) =
        common::make_request(app.clone(), "POST", "/api/boards", Some(body), Some(&token)).await;
    let board: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let board_id = board["id"].as_str().unwrap();

    let update_body = json!({ "name": "New Name" }).to_string();
    let (status, resp) = common::make_request(
        app,
        "PATCH",
        &format!("/api/boards/{}", board_id),
        Some(update_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Update board failed: {}", resp);
    let updated: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(updated["name"], "New Name");
}

#[tokio::test]
async fn test_delete_board() {
    let (app, token) = test_app().await;

    let body = json!({ "name": "To Delete" }).to_string();
    let (_, resp) =
        common::make_request(app.clone(), "POST", "/api/boards", Some(body), Some(&token)).await;
    let board: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let board_id = board["id"].as_str().unwrap();

    let (status, _) = common::make_request(
        app.clone(),
        "DELETE",
        &format!("/api/boards/{}", board_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, resp) =
        common::make_request(app, "GET", "/api/boards", None, Some(&token)).await;
    let boards: Vec<serde_json::Value> = serde_json::from_str(&resp).unwrap();
    assert!(
        !boards.iter().any(|b| b["id"].as_str() == Some(board_id)),
        "Deleted board should not appear in list"
    );
}

#[tokio::test]
async fn test_reorder_board() {
    let (app, token) = test_app().await;

    let body = json!({ "name": "Reorder Me" }).to_string();
    let (_, resp) =
        common::make_request(app.clone(), "POST", "/api/boards", Some(body), Some(&token)).await;
    let board: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let board_id = board["id"].as_str().unwrap();

    let reorder_body = json!({ "position": 5000 }).to_string();
    let (status, resp) = common::make_request(
        app,
        "PATCH",
        &format!("/api/boards/{}/reorder", board_id),
        Some(reorder_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Reorder failed: {}", resp);
    let reordered: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(reordered["position"], 5000);
}

// ---------------------------------------------------------------------------
// Subtasks — CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_subtask_crud() {
    let (app, token) = test_app().await;

    let card_body = json!({ "title": "Subtask Parent" }).to_string();
    let (_, resp) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(card_body),
        Some(&token),
    )
    .await;
    let card: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let card_id = card["id"].as_str().unwrap();

    let sub_body = json!({
        "title": "Write tests",
        "phase": "Phase 1",
        "phase_order": 1
    })
    .to_string();
    let (status, resp) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/subtasks", card_id),
        Some(sub_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "Create subtask failed: {}", resp);
    let subtask: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(subtask["title"], "Write tests");
    assert_eq!(subtask["phase"], "Phase 1");
    assert_eq!(subtask["completed"], false);
    let subtask_id = subtask["id"].as_str().unwrap();

    let update_body = json!({
        "completed": true,
        "title": "Write tests (done)"
    })
    .to_string();
    let (status, resp) = common::make_request(
        app.clone(),
        "PATCH",
        &format!("/api/subtasks/{}", subtask_id),
        Some(update_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Update subtask failed: {}", resp);
    let updated: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(updated["completed"], true);
    assert_eq!(updated["title"], "Write tests (done)");

    let (_, resp) = common::make_request(
        app.clone(),
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    let card_detail: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let subtasks = card_detail["subtasks"].as_array().unwrap();
    assert_eq!(subtasks.len(), 1);
    assert_eq!(subtasks[0]["completed"], true);

    let (status, _) = common::make_request(
        app.clone(),
        "DELETE",
        &format!("/api/subtasks/{}", subtask_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, resp) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    let card_detail: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let subtasks = card_detail["subtasks"].as_array().unwrap();
    assert_eq!(subtasks.len(), 0);
}

#[tokio::test]
async fn test_subtask_phase_ordering() {
    let (app, token) = test_app().await;

    let card_body = json!({ "title": "Phase Card" }).to_string();
    let (_, resp) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(card_body),
        Some(&token),
    )
    .await;
    let card: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let card_id = card["id"].as_str().unwrap();

    for (title, phase, order) in [
        ("Task B", "Phase 2", 2),
        ("Task A", "Phase 1", 1),
        ("Task C", "Phase 2", 2),
    ] {
        let body = json!({
            "title": title,
            "phase": phase,
            "phase_order": order
        })
        .to_string();
        common::make_request(
            app.clone(),
            "POST",
            &format!("/api/cards/{}/subtasks", card_id),
            Some(body),
            Some(&token),
        )
        .await;
    }

    let (_, resp) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    let card_detail: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let subtasks = card_detail["subtasks"].as_array().unwrap();
    assert_eq!(subtasks.len(), 3);
}

// ---------------------------------------------------------------------------
// Comments — CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_comment_crud() {
    let (app, token) = test_app().await;

    let card_body = json!({ "title": "Comment Card" }).to_string();
    let (_, resp) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(card_body),
        Some(&token),
    )
    .await;
    let card: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let card_id = card["id"].as_str().unwrap();

    let (status, resp) = common::make_request(
        app.clone(),
        "GET",
        &format!("/api/cards/{}/comments", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let comments: Vec<serde_json::Value> = serde_json::from_str(&resp).unwrap();
    assert_eq!(comments.len(), 0);

    let comment_body = json!({
        "content": "Looks good!",
        "author": "Steven"
    })
    .to_string();
    let (status, resp) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/comments", card_id),
        Some(comment_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "Create comment failed: {}", resp);
    let comment: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(comment["content"], "Looks good!");
    assert_eq!(comment["author"], "Steven");
    let comment_id = comment["id"].as_str().unwrap();

    let update_body = json!({ "content": "Actually, needs changes" }).to_string();
    let (status, resp) = common::make_request(
        app.clone(),
        "PATCH",
        &format!("/api/comments/{}", comment_id),
        Some(update_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Update comment failed: {}", resp);
    let updated: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(updated["content"], "Actually, needs changes");

    let (_, resp) = common::make_request(
        app.clone(),
        "GET",
        &format!("/api/cards/{}/comments", card_id),
        None,
        Some(&token),
    )
    .await;
    let comments: Vec<serde_json::Value> = serde_json::from_str(&resp).unwrap();
    assert_eq!(comments.len(), 1);

    let (status, _) = common::make_request(
        app.clone(),
        "DELETE",
        &format!("/api/comments/{}", comment_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, resp) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}/comments", card_id),
        None,
        Some(&token),
    )
    .await;
    let comments: Vec<serde_json::Value> = serde_json::from_str(&resp).unwrap();
    assert_eq!(comments.len(), 0);
}

#[tokio::test]
async fn test_comment_default_author() {
    let (app, token) = test_app().await;

    let card_body = json!({ "title": "Default Author Card" }).to_string();
    let (_, resp) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(card_body),
        Some(&token),
    )
    .await;
    let card: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let card_id = card["id"].as_str().unwrap();

    let comment_body = json!({ "content": "No author specified" }).to_string();
    let (status, resp) = common::make_request(
        app,
        "POST",
        &format!("/api/cards/{}/comments", card_id),
        Some(comment_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let comment: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert!(
        comment["author"].is_string(),
        "Author should have a default value"
    );
}

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_list_labels() {
    let (app, token) = test_app().await;

    let (status, resp) =
        common::make_request(app, "GET", "/api/labels", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let labels: Vec<serde_json::Value> = serde_json::from_str(&resp).unwrap();
    assert!(labels.len() >= 5, "Should have 5 default seeded labels, got {}", labels.len());
}

#[tokio::test]
async fn test_add_and_remove_label() {
    let (app, token) = test_app().await;

    let (_, resp) =
        common::make_request(app.clone(), "GET", "/api/labels", None, Some(&token)).await;
    let labels: Vec<serde_json::Value> = serde_json::from_str(&resp).unwrap();
    let label_id = labels[0]["id"].as_str().unwrap();

    let card_body = json!({ "title": "Label Card" }).to_string();
    let (_, resp) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(card_body),
        Some(&token),
    )
    .await;
    let card: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let card_id = card["id"].as_str().unwrap();

    let (status, _) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/labels/{}", card_id, label_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, resp) = common::make_request(
        app.clone(),
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    let card_detail: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let card_labels = card_detail["labels"].as_array().unwrap();
    assert_eq!(card_labels.len(), 1);
    assert_eq!(card_labels[0]["id"].as_str().unwrap(), label_id);

    let (status, _) = common::make_request(
        app.clone(),
        "DELETE",
        &format!("/api/cards/{}/labels/{}", card_id, label_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, resp) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    let card_detail: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let card_labels = card_detail["labels"].as_array().unwrap();
    assert_eq!(card_labels.len(), 0);
}

// ---------------------------------------------------------------------------
// Board Settings
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_board_settings_get_and_update() {
    let (app, token) = test_app().await;

    let (status, resp) = common::make_request(
        app.clone(),
        "GET",
        "/api/boards/default/settings",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Get settings failed: {}", resp);
    let settings: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(settings["board_id"], "default");

    let update_body = json!({
        "tech_stack": "Rust, React, SQLite",
        "codebase_path": "/home/steven/kanban",
        "code_conventions": "Use snake_case for Rust, camelCase for TS"
    })
    .to_string();
    let (status, resp) = common::make_request(
        app.clone(),
        "PUT",
        "/api/boards/default/settings",
        Some(update_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Update settings failed: {}", resp);
    let updated: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(updated["tech_stack"], "Rust, React, SQLite");
    assert_eq!(updated["codebase_path"], "/home/steven/kanban");

    let (_, resp) = common::make_request(
        app,
        "GET",
        "/api/boards/default/settings",
        None,
        Some(&token),
    )
    .await;
    let settings: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(settings["tech_stack"], "Rust, React, SQLite");
}

// ---------------------------------------------------------------------------
// Settings (key-value)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_settings_get_and_set() {
    let (app, token) = test_app().await;

    let set_body = json!({ "value": "3" }).to_string();
    let (status, resp) = common::make_request(
        app.clone(),
        "PUT",
        "/api/settings/ai_concurrency",
        Some(set_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Set setting failed: {}", resp);
    let setting: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(setting["key"], "ai_concurrency");
    assert_eq!(setting["value"], "3");

    let (status, resp) = common::make_request(
        app,
        "GET",
        "/api/settings/ai_concurrency",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let setting: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(setting["value"], "3");
}

#[tokio::test]
async fn test_settings_not_found() {
    let (app, token) = test_app().await;
    let (status, _) = common::make_request(
        app,
        "GET",
        "/api/settings/nonexistent_key",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Card versions (snapshot on update)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_card_version_history() {
    let (app, token) = test_app().await;

    let card_body = json!({ "title": "Version Card" }).to_string();
    let (_, resp) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(card_body),
        Some(&token),
    )
    .await;
    let card: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let card_id = card["id"].as_str().unwrap();

    let update_body = json!({ "title": "Version Card v2" }).to_string();
    common::make_request(
        app.clone(),
        "PATCH",
        &format!("/api/cards/{}", card_id),
        Some(update_body),
        Some(&token),
    )
    .await;

    let (status, resp) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}/versions", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Get versions failed: {}", resp);
    let versions: Vec<serde_json::Value> = serde_json::from_str(&resp).unwrap();
    assert!(
        !versions.is_empty(),
        "Should have at least one version snapshot after update"
    );
}

// ---------------------------------------------------------------------------
// Full lifecycle E2E
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_card_lifecycle() {
    let (app, token) = test_app().await;

    let card_body = json!({
        "title": "E2E Lifecycle Card",
        "description": "Full lifecycle test"
    })
    .to_string();
    let (status, resp) = common::make_request(
        app.clone(),
        "POST",
        "/api/cards",
        Some(card_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let card: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let card_id = card["id"].as_str().unwrap();
    assert_eq!(card["stage"], "backlog");

    for title in ["Design API", "Implement handler", "Write tests"] {
        let sub_body = json!({ "title": title }).to_string();
        let (status, _) = common::make_request(
            app.clone(),
            "POST",
            &format!("/api/cards/{}/subtasks", card_id),
            Some(sub_body),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    let comment_body = json!({
        "content": "Starting work on this",
        "author": "AI Agent"
    })
    .to_string();
    let (status, _) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/comments", card_id),
        Some(comment_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, resp) =
        common::make_request(app.clone(), "GET", "/api/labels", None, Some(&token)).await;
    let labels: Vec<serde_json::Value> = serde_json::from_str(&resp).unwrap();
    let label_id = labels[0]["id"].as_str().unwrap();
    let (status, _) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/labels/{}", card_id, label_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let move_body = json!({ "stage": "plan", "position": 1000 }).to_string();
    let (status, _) = common::make_request(
        app.clone(),
        "PATCH",
        &format!("/api/cards/{}/move", card_id),
        Some(move_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let move_body = json!({ "stage": "todo", "position": 1000 }).to_string();
    let (status, _) = common::make_request(
        app.clone(),
        "PATCH",
        &format!("/api/cards/{}/move", card_id),
        Some(move_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, resp) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    let final_card: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(final_card["stage"], "todo");
    assert_eq!(final_card["subtasks"].as_array().unwrap().len(), 3);
    assert_eq!(final_card["comments"].as_array().unwrap().len(), 1);
    assert_eq!(final_card["labels"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_list_notifications_empty() {
    let (app, token) = test_app().await;

    let (status, body) =
        common::make_request(app, "GET", "/api/notifications", None, Some(&token)).await;

    assert_eq!(status, StatusCode::OK);
    let notifications: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(notifications.len(), 0);
}

#[tokio::test]
async fn test_notification_crud() {
    let (app, token, pool) = test_app_with_pool().await;
    let notification_id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO notifications (id, user_id, notification_type, title, message, card_id, board_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&notification_id)
    .bind(Option::<String>::None)
    .bind("review_requested")
    .bind("Review requested: Integration Test Card")
    .bind("Card is ready for review")
    .bind(Option::<String>::None)
    .bind(Some("default"))
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) =
        common::make_request(app.clone(), "GET", "/api/notifications", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let notifications: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert!(notifications.iter().any(|n| n["id"] == notification_id));

    let (status, body) = common::make_request(
        app.clone(),
        "PATCH",
        &format!("/api/notifications/{}/read", notification_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let marked: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(marked["id"], notification_id);
    assert_eq!(marked["is_read"], true);

    let (status, body) = common::make_request(
        app.clone(),
        "GET",
        "/api/notifications?unread_only=true",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let unread: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert!(!unread.iter().any(|n| n["id"] == notification_id));

    let (status, _) = common::make_request(
        app.clone(),
        "DELETE",
        &format!("/api/notifications/{}", notification_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) =
        common::make_request(app, "GET", "/api/notifications", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let notifications: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(notifications.len(), 0);
}

#[tokio::test]
async fn test_mark_all_notifications_read() {
    let (app, token, pool) = test_app_with_pool().await;

    for idx in 1..=3 {
        sqlx::query(
            "INSERT INTO notifications (id, user_id, notification_type, title, message, card_id, board_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(Option::<String>::None)
        .bind("card_stage_changed")
        .bind(format!("Notification {}", idx))
        .bind("Seeded unread notification")
        .bind(Option::<String>::None)
        .bind(Some("default"))
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    }

    let (status, body) =
        common::make_request(app.clone(), "POST", "/api/notifications/read-all", None, Some(&token))
            .await;
    assert_eq!(status, StatusCode::OK);
    let resp: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(resp["marked_read"], 3);

    let (status, body) = common::make_request(
        app,
        "GET",
        "/api/notifications?unread_only=true",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let unread: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(unread.len(), 0);
}

#[tokio::test]
async fn test_delete_nonexistent_notification() {
    let (app, token) = test_app().await;

    let (status, _) = common::make_request(
        app,
        "DELETE",
        "/api/notifications/nonexistent-id",
        None,
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_move_card_to_review_creates_notification() {
    let (app, token) = test_app().await;

    let create_body = json!({
        "title": "Review Notification Card",
        "description": "Card that should trigger review notification"
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
    let card_id = card["id"].as_str().unwrap();

    for stage in ["plan", "todo", "in_progress", "review"] {
        let move_body = json!({ "stage": stage, "position": 1000 }).to_string();
        let (status, body) = common::make_request(
            app.clone(),
            "PATCH",
            &format!("/api/cards/{}/move", card_id),
            Some(move_body),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "Move to {} failed: {}", stage, body);
    }

    let (status, body) =
        common::make_request(app, "GET", "/api/notifications", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let notifications: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();

    assert!(notifications.iter().any(|n| {
        n["notification_type"] == "review_requested"
            && n["card_id"] == card_id
            && n["title"].as_str().unwrap_or_default().contains("Review requested")
    }));
}
