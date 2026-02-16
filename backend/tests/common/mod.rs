use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use sqlx::SqlitePool;
use tower::ServiceExt;
use uuid::Uuid;

pub async fn make_multipart_request(
    app: Router,
    uri: &str,
    field_name: &str,
    file_name: &str,
    content_type: &str,
    file_bytes: Vec<u8>,
    auth_token: Option<&str>,
) -> (StatusCode, String) {
    let boundary = "----TestBoundary7MA4YWxkTrZu0gW";
    let mut body_bytes = Vec::new();

    body_bytes.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body_bytes.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            field_name, file_name
        )
        .as_bytes(),
    );
    body_bytes
        .extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
    body_bytes.extend_from_slice(&file_bytes);
    body_bytes.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    let mut request = Request::builder().uri(uri).method("POST").header(
        "content-type",
        format!("multipart/form-data; boundary={}", boundary),
    );

    if let Some(token) = auth_token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let request = request
        .body(Body::from(body_bytes))
        .expect("Failed to build multipart request");

    let response = app
        .oneshot(request)
        .await
        .expect("Multipart request failed");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body_str = String::from_utf8(body.to_vec()).unwrap_or_default();

    (status, body_str)
}

pub async fn make_raw_request(
    app: Router,
    method: &str,
    uri: &str,
    auth_token: Option<&str>,
) -> (StatusCode, Vec<u8>, Option<String>) {
    let mut request = Request::builder().uri(uri).method(method);

    if let Some(token) = auth_token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let request = request
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Request failed");
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    (status, body.to_vec(), content_type)
}

pub async fn setup_test_db_with_user_id() -> (SqlitePool, String, String) {
    let (pool, token, user_id) = setup_test_db_inner().await;
    (pool, token, user_id)
}

async fn setup_test_db_inner() -> (SqlitePool, String, String) {
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

    (pool, token, user_id)
}

pub async fn setup_test_db() -> (SqlitePool, String) {
    let (pool, token, _) = setup_test_db_inner().await;
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
