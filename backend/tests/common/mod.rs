use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use sqlx::SqlitePool;
use tower::ServiceExt;

pub async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    sqlx::query(
        r#"
        CREATE TABLE cards (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            stage TEXT NOT NULL DEFAULT 'backlog',
            position INTEGER NOT NULL DEFAULT 0,
            priority TEXT NOT NULL DEFAULT 'medium',
            working_directory TEXT NOT NULL DEFAULT '.',
            plan_path TEXT,
            ai_session_id TEXT,
            ai_status TEXT NOT NULL DEFAULT 'idle',
            ai_progress TEXT NOT NULL DEFAULT '{}',
            linked_documents TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE subtasks (
            id TEXT PRIMARY KEY,
            card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            completed INTEGER NOT NULL DEFAULT 0,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE labels (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL
        );

        CREATE TABLE card_labels (
            card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
            label_id TEXT NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
            PRIMARY KEY (card_id, label_id)
        );

        CREATE TABLE comments (
            id TEXT PRIMARY KEY,
            card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
            author TEXT NOT NULL DEFAULT 'user',
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        INSERT INTO labels (id, name, color) VALUES
            ('lbl-bug', 'Bug', '#f44336'),
            ('lbl-feature', 'Feature', '#4caf50'),
            ('lbl-improvement', 'Improvement', '#2196f3'),
            ('lbl-docs', 'Documentation', '#ff9800'),
            ('lbl-urgent', 'Urgent', '#e91e63');
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create test schema");

    pool
}

pub async fn make_request(
    app: Router,
    method: &str,
    uri: &str,
    body: Option<String>,
) -> (StatusCode, String) {
    let mut request = Request::builder().uri(uri).method(method);

    if body.is_some() {
        request = request.header("content-type", "application/json");
    }

    let request = request
        .body(Body::from(body.unwrap_or_default()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    (status, body_str)
}
