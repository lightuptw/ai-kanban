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
async fn test_session_mapping_crud() {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();

    let config = Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
    });

    let state = kanban_backend::api::state::AppState {
        db: Some(pool.clone()),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);

    // Create a card to associate mappings with
    let create_body = json!({
        "title": "Session Mapping Test Card"
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
    let card: serde_json::Value = serde_json::from_str(&body)
        .expect("card creation response should be valid JSON");
    let card_id = card["id"].as_str().expect("card should have an id");

    // Insert session mappings directly via SQL (service not in test harness)
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO session_mappings (child_session_id, card_id, parent_session_id, agent_type, description, created_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind("child-sess-1")
    .bind(card_id)
    .bind("parent-sess-1")
    .bind("explore")
    .bind("Exploring codebase")
    .bind(&now)
    .execute(&pool)
    .await
    .expect("inserting first session mapping should succeed");

    sqlx::query(
        "INSERT INTO session_mappings (child_session_id, card_id, parent_session_id, agent_type, description, created_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind("child-sess-2")
    .bind(card_id)
    .bind("parent-sess-1")
    .bind("librarian")
    .bind("Looking up docs")
    .bind(&now)
    .execute(&pool)
    .await
    .expect("inserting second session mapping should succeed");

    // Verify lookup by child_session_id
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT card_id FROM session_mappings WHERE child_session_id = ?"
    )
    .bind("child-sess-1")
    .fetch_optional(&pool)
    .await
    .expect("lookup by child_session_id should not fail");
    assert_eq!(
        row.expect("should find mapping for child-sess-1").0,
        card_id
    );

    // Verify list by card_id returns both mappings in order
    let mappings: Vec<(String, String, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT child_session_id, card_id, parent_session_id, agent_type, description, created_at FROM session_mappings WHERE card_id = ? ORDER BY created_at ASC"
    )
    .bind(card_id)
    .fetch_all(&pool)
    .await
    .expect("listing mappings by card_id should not fail");
    assert_eq!(mappings.len(), 2);
    assert_eq!(mappings[0].0, "child-sess-1");
    assert_eq!(mappings[0].3.as_deref(), Some("explore"));
    assert_eq!(mappings[1].0, "child-sess-2");
    assert_eq!(mappings[1].3.as_deref(), Some("librarian"));

    // Verify lookup for non-existent child session returns None
    let missing: Option<(String,)> = sqlx::query_as(
        "SELECT card_id FROM session_mappings WHERE child_session_id = ?"
    )
    .bind("nonexistent-session")
    .fetch_optional(&pool)
    .await
    .expect("lookup for missing session should not fail");
    assert!(missing.is_none(), "non-existent session should return None");

    // Delete mappings by card_id
    sqlx::query("DELETE FROM session_mappings WHERE card_id = ?")
        .bind(card_id)
        .execute(&pool)
        .await
        .expect("deleting mappings by card_id should succeed");

    // Verify mappings are gone
    let after_delete: Vec<(String,)> = sqlx::query_as(
        "SELECT child_session_id FROM session_mappings WHERE card_id = ?"
    )
    .bind(card_id)
    .fetch_all(&pool)
    .await
    .expect("listing after delete should not fail");
    assert!(after_delete.is_empty(), "all mappings should be deleted");
}

#[tokio::test]
async fn test_agent_activity_endpoint() {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();

    let config = Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
    });

    let state = kanban_backend::api::state::AppState {
        db: Some(pool.clone()),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);

    // Create a card
    let create_body = json!({
        "title": "Agent Activity Test Card"
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
    let card: serde_json::Value = serde_json::from_str(&body)
        .expect("card creation response should be valid JSON");
    let card_id = card["id"].as_str().expect("card should have an id");

    // Insert agent_logs directly
    let early_time = "2026-01-01T00:00:00+00:00";
    let late_time = "2026-01-02T00:00:00+00:00";

    sqlx::query(
        "INSERT INTO agent_logs (id, card_id, session_id, event_type, agent, content, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("log-1")
    .bind(card_id)
    .bind("sess-1")
    .bind("message")
    .bind("build")
    .bind("Building feature")
    .bind("{}")
    .bind(early_time)
    .execute(&pool)
    .await
    .expect("inserting first agent log should succeed");

    sqlx::query(
        "INSERT INTO agent_logs (id, card_id, session_id, event_type, agent, content, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("log-2")
    .bind(card_id)
    .bind("sess-1")
    .bind("tool_call")
    .bind("build")
    .bind("Running tests")
    .bind("{}")
    .bind(late_time)
    .execute(&pool)
    .await
    .expect("inserting second agent log should succeed");

    sqlx::query(
        "INSERT INTO agent_logs (id, card_id, session_id, event_type, agent, content, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("log-3")
    .bind(card_id)
    .bind("sess-2")
    .bind("message")
    .bind("explore")
    .bind("Searching files")
    .bind("{}")
    .bind(late_time)
    .execute(&pool)
    .await
    .expect("inserting third agent log should succeed");

    // Insert session mappings
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO session_mappings (child_session_id, card_id, parent_session_id, agent_type, description, created_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind("child-activity-1")
    .bind(card_id)
    .bind("parent-activity-1")
    .bind("explore")
    .bind("Exploring for activity test")
    .bind(&now)
    .execute(&pool)
    .await
    .expect("inserting session mapping should succeed");

    // Hit the agent-activity endpoint
    let (status, body) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}/agent-activity", card_id),
        None,
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let activity: serde_json::Value = serde_json::from_str(&body)
        .expect("agent-activity response should be valid JSON");

    // Verify response shape
    assert_eq!(activity["card_id"], card_id);
    assert!(activity["agents"].is_array(), "agents should be an array");
    assert!(activity["session_mappings"].is_array(), "session_mappings should be an array");

    // Verify agents aggregation: build has 2 events, explore has 1
    let agents = activity["agents"].as_array()
        .expect("agents should be a JSON array");
    assert_eq!(agents.len(), 2, "should have 2 distinct agents");

    // Agents are ordered by first_seen ASC; build appeared first
    assert_eq!(agents[0]["agent_type"], "build");
    assert_eq!(agents[0]["event_count"], 2);
    assert_eq!(agents[0]["first_seen"], early_time);
    assert_eq!(agents[0]["last_seen"], late_time);

    assert_eq!(agents[1]["agent_type"], "explore");
    assert_eq!(agents[1]["event_count"], 1);

    // Verify session_mappings in response
    let sm = activity["session_mappings"].as_array()
        .expect("session_mappings should be a JSON array");
    assert_eq!(sm.len(), 1);
    assert_eq!(sm[0]["child_session_id"], "child-activity-1");
    assert_eq!(sm[0]["card_id"], card_id);
    assert_eq!(sm[0]["agent_type"], "explore");
}

#[tokio::test]
async fn test_session_mapping_cascade_delete() {
    let (pool, token) = common::setup_test_db().await;
    let (sse_tx, _) = tokio::sync::broadcast::channel(100);
    let http_client = reqwest::Client::new();

    let config = Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
    });

    let state = kanban_backend::api::state::AppState {
        db: Some(pool.clone()),
        sse_tx,
        http_client,
        config: config.clone(),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);

    // Create a card
    let create_body = json!({
        "title": "Cascade Delete Test Card"
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
    let card: serde_json::Value = serde_json::from_str(&body)
        .expect("card creation response should be valid JSON");
    let card_id = card["id"].as_str().expect("card should have an id");

    // Insert session mappings for this card
    let now = chrono::Utc::now().to_rfc3339();
    for i in 1..=3 {
        sqlx::query(
            "INSERT INTO session_mappings (child_session_id, card_id, parent_session_id, agent_type, description, created_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(format!("cascade-child-{}", i))
        .bind(card_id)
        .bind("cascade-parent")
        .bind("build")
        .bind(format!("Cascade test mapping {}", i))
        .bind(&now)
        .execute(&pool)
        .await
        .expect("inserting cascade test mapping should succeed");
    }

    // Verify mappings exist
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM session_mappings WHERE card_id = ?"
    )
    .bind(card_id)
    .fetch_one(&pool)
    .await
    .expect("counting mappings should not fail");
    assert_eq!(count.0, 3, "should have 3 mappings before cascade delete");

    // Delete the card via API (triggers CASCADE)
    let (status, _) = common::make_request(
        app.clone(),
        "DELETE",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify all session_mappings for that card are gone (CASCADE)
    let remaining: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM session_mappings WHERE card_id = ?"
    )
    .bind(card_id)
    .fetch_one(&pool)
    .await
    .expect("counting remaining mappings should not fail");
    assert_eq!(
        remaining.0, 0,
        "CASCADE delete should remove all session_mappings when card is deleted"
    );

    // Double-check individual mappings are truly gone
    let orphan: Option<(String,)> = sqlx::query_as(
        "SELECT card_id FROM session_mappings WHERE child_session_id = ?"
    )
    .bind("cascade-child-1")
    .fetch_optional(&pool)
    .await
    .expect("checking orphan mapping should not fail");
    assert!(
        orphan.is_none(),
        "individual mapping should not survive card deletion"
    );
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
