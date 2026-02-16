use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use sqlx::SqlitePool;
use tower::ServiceExt;
use uuid::Uuid;

pub async fn setup_test_db() -> (SqlitePool, String) {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run test migrations");

    let user_id = Uuid::new_v4().to_string();
    let tenant_id = Uuid::new_v4().to_string();
    let password_hash = kanban_backend::auth::password::hash_password("TestPass123")
        .expect("Failed to hash test password");
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, tenant_id, username, nickname, first_name, last_name, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(&tenant_id)
    .bind("test_user")
    .bind("Test User")
    .bind("")
    .bind("")
    .bind("")
    .bind(&password_hash)
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .expect("Failed to seed test user");

    let signing_key = kanban_backend::auth::jwt::get_or_create_signing_key(&pool)
        .await
        .expect("Failed to create JWT signing key");
    let token = kanban_backend::auth::jwt::create_token(&signing_key, &user_id, &tenant_id)
        .expect("Failed to create JWT token");

    sqlx::query(
        "INSERT OR IGNORE INTO boards (id, name, created_at, updated_at) VALUES ('default', 'Test Board', datetime('now'), datetime('now'))",
    )
    .execute(&pool)
    .await
    .expect("Failed to ensure default board");

    (pool, token)
}

pub async fn make_request(
    app: Router,
    method: &str,
    uri: &str,
    body: Option<String>,
    auth_token: Option<&str>,
) -> (StatusCode, String) {
    let mut request = Request::builder().uri(uri).method(method);

    if body.is_some() {
        request = request.header("content-type", "application/json");
    }

    if let Some(token) = auth_token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let request = request
        .body(Body::from(body.unwrap_or_default()))
        .expect("Failed to build test request");

    let response = app
        .oneshot(request)
        .await
        .expect("Test request failed unexpectedly");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body_str = String::from_utf8(body.to_vec()).expect("Response body is not valid UTF-8");

    (status, body_str)
}
