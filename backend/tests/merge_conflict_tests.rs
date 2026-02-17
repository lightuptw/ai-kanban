mod common;

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use axum::http::StatusCode;
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

use kanban_backend::services::GitWorktreeService;

fn git(repo_path: &str, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .expect("git command should execute");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "git command failed in {}: git {}\n{}",
            repo_path,
            args.join(" "),
            stderr.trim()
        );
    }

    String::from_utf8(output.stdout).expect("git output should be valid UTF-8")
}

fn current_branch(repo_path: &str) -> String {
    git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .trim()
        .to_string()
}

fn abort_merge_if_needed(repo_path: &str) {
    if Path::new(repo_path).join(".git/MERGE_HEAD").exists() {
        let _ = Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(repo_path)
            .output();
    }
}

fn create_test_repo_with_conflict() -> (TempDir, String, String) {
    let tmp = TempDir::new().expect("temporary directory should be created");
    let repo_path = tmp.path().to_string_lossy().to_string();

    git(&repo_path, &["init"]);
    git(&repo_path, &["config", "user.email", "test@test.com"]);
    git(&repo_path, &["config", "user.name", "Test User"]);

    std::fs::write(tmp.path().join("file.txt"), "line1\nline2\nline3\n")
        .expect("initial file should be written");
    git(&repo_path, &["add", "."]);
    git(&repo_path, &["commit", "-m", "initial"]);

    let default_branch = current_branch(&repo_path);
    let branch_name = "ai/test-branch";

    git(&repo_path, &["checkout", "-b", branch_name]);
    std::fs::write(
        tmp.path().join("file.txt"),
        "line1\nmodified-by-branch\nline3\n",
    )
    .expect("branch file update should be written");
    git(&repo_path, &["add", "."]);
    git(&repo_path, &["commit", "-m", "branch change"]);

    git(&repo_path, &["checkout", default_branch.as_str()]);
    std::fs::write(
        tmp.path().join("file.txt"),
        "line1\nmodified-by-main\nline3\n",
    )
    .expect("default branch file update should be written");
    git(&repo_path, &["add", "."]);
    git(&repo_path, &["commit", "-m", "main change"]);

    (tmp, repo_path, branch_name.to_string())
}

fn test_config() -> Arc<kanban_backend::config::Config> {
    Arc::new(kanban_backend::config::Config {
        port: 3000,
        database_url: "sqlite::memory:".to_string(),
        opencode_url: "http://localhost:4096".to_string(),
        frontend_dir: "../frontend/dist".to_string(),
        cors_origin: "http://localhost:5173".to_string(),
    })
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
        merge_locks: Arc::new(Mutex::new(HashSet::new())),
    };

    let app = kanban_backend::api::routes::create_router(state, &config);
    (app, token, pool)
}

async fn seed_merge_ready_card(pool: &sqlx::SqlitePool, repo_path: &str, branch_name: &str) -> String {
    sqlx::query("INSERT OR REPLACE INTO board_settings (board_id, codebase_path, updated_at) VALUES ('default', ?, ?)")
        .bind(repo_path)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(pool)
        .await
        .expect("board settings should be inserted");

    let card_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query("INSERT INTO cards (id, title, description, stage, position, priority, working_directory, ai_status, ai_progress, linked_documents, created_at, updated_at, board_id, branch_name, worktree_path) VALUES (?, ?, ?, 'review', 1000, 'medium', '.', 'idle', '{}', '[]', ?, ?, 'default', ?, '')")
        .bind(&card_id)
        .bind("Merge Conflict Test")
        .bind("Testing merge conflict flow")
        .bind(&now)
        .bind(&now)
        .bind(branch_name)
        .execute(pool)
        .await
        .expect("merge-ready card should be inserted");

    card_id
}

async fn start_conflicting_merge(
    app: axum::Router,
    token: &str,
    card_id: &str,
) -> serde_json::Value {
    let (status, body) = common::make_request(
        app,
        "POST",
        &format!("/api/cards/{}/merge", card_id),
        None,
        Some(token),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "merge endpoint failed: {}", body);
    let merge_result: serde_json::Value =
        serde_json::from_str(&body).expect("merge response should be valid JSON");
    assert_eq!(merge_result["success"], false);
    assert!(merge_result["conflict_detail"].is_object());

    merge_result
}

#[test]
fn test_merge_branch_keep_conflicts_preserves_state() {
    let (_tmp, repo_path, branch_name) = create_test_repo_with_conflict();

    let result = GitWorktreeService::merge_branch(&repo_path, &branch_name, true, "", "")
        .expect("merge should return a result");

    assert!(!result.success, "merge should fail due to conflict");
    assert!(
        !result.conflicts.is_empty(),
        "merge should report conflicting files"
    );
    assert!(
        result.conflict_detail.is_some(),
        "keep_conflicts=true should include conflict detail"
    );
    assert!(Path::new(&repo_path).join(".git/MERGE_HEAD").exists());
    assert!(GitWorktreeService::is_merge_in_progress(&repo_path));

    abort_merge_if_needed(&repo_path);
}

#[test]
fn test_merge_branch_no_keep_conflicts_aborts() {
    let (_tmp, repo_path, branch_name) = create_test_repo_with_conflict();

    let result = GitWorktreeService::merge_branch(&repo_path, &branch_name, false, "", "")
        .expect("merge should return a result");

    assert!(!result.success, "merge should fail due to conflict");
    assert!(
        !result.conflicts.is_empty(),
        "merge should report conflicting files"
    );
    assert!(
        result.conflict_detail.is_none(),
        "keep_conflicts=false should not return conflict detail"
    );
    assert!(!Path::new(&repo_path).join(".git/MERGE_HEAD").exists());
    assert!(!GitWorktreeService::is_merge_in_progress(&repo_path));
}

#[test]
fn test_get_conflict_details_extracts_content() {
    let (_tmp, repo_path, branch_name) = create_test_repo_with_conflict();

    let merge_result = GitWorktreeService::merge_branch(&repo_path, &branch_name, true, "", "")
        .expect("merge should run for conflict detail test");
    assert!(!merge_result.success, "merge should fail with conflicts");

    let detail = GitWorktreeService::get_conflict_details(&repo_path)
        .expect("conflict details should be retrievable");

    assert!(detail.merge_in_progress);
    assert_eq!(detail.files.len(), 1, "expected one conflict file");

    let file = &detail.files[0];
    assert_eq!(file.path, "file.txt");
    assert_eq!(file.conflict_type, "both-modified");
    assert!(!file.is_binary);
    assert!(file.ours_content.is_some());
    assert!(file.theirs_content.is_some());
    assert!(file.base_content.is_some());

    let ours = file
        .ours_content
        .as_ref()
        .expect("ours content should exist for conflict");
    let theirs = file
        .theirs_content
        .as_ref()
        .expect("theirs content should exist for conflict");
    let base = file
        .base_content
        .as_ref()
        .expect("base content should exist for conflict");

    assert!(ours.contains("modified-by-main"));
    assert!(theirs.contains("modified-by-branch"));
    assert!(base.contains("line2"));

    abort_merge_if_needed(&repo_path);
}

#[tokio::test]
async fn test_conflict_resolution_http_flow_e2e_moves_card_to_done() {
    let (app, token, pool) = test_app_with_pool().await;
    let (_tmp, repo_path, branch_name) = create_test_repo_with_conflict();
    let card_id = seed_merge_ready_card(&pool, &repo_path, &branch_name).await;

    let _merge_result = start_conflicting_merge(app.clone(), &token, &card_id).await;

    let (status, body) = common::make_request(
        app.clone(),
        "GET",
        &format!("/api/cards/{}/conflicts", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get conflicts failed: {}", body);
    let conflicts: serde_json::Value =
        serde_json::from_str(&body).expect("conflicts response should be valid JSON");
    assert_eq!(conflicts["merge_in_progress"], true);
    assert_eq!(conflicts["files"][0]["path"], "file.txt");

    let resolve_body = json!({
        "resolutions": [
            {
                "file_path": "file.txt",
                "choice": "ours"
            }
        ]
    })
    .to_string();
    let (status, body) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/resolve-conflicts", card_id),
        Some(resolve_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "resolve conflicts failed: {}", body);
    let post_resolve: serde_json::Value =
        serde_json::from_str(&body).expect("resolve response should be valid JSON");
    assert_eq!(post_resolve["files"].as_array().map_or(0, Vec::len), 0);

    let (status, body) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/complete-merge", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "complete merge failed: {}", body);
    let complete_result: serde_json::Value =
        serde_json::from_str(&body).expect("complete merge response should be valid JSON");
    assert_eq!(complete_result["success"], true);

    let (status, body) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get card failed: {}", body);
    let card: serde_json::Value = serde_json::from_str(&body).expect("card response should be JSON");
    assert_eq!(card["stage"], "done");
    assert!(!Path::new(&repo_path).join(".git/MERGE_HEAD").exists());
}

#[tokio::test]
async fn test_resolve_conflicts_endpoint_theirs_choice() {
    let (app, token, pool) = test_app_with_pool().await;
    let (_tmp, repo_path, branch_name) = create_test_repo_with_conflict();
    let card_id = seed_merge_ready_card(&pool, &repo_path, &branch_name).await;

    let _merge_result = start_conflicting_merge(app.clone(), &token, &card_id).await;

    let resolve_body = json!({
        "resolutions": [
            {
                "file_path": "file.txt",
                "choice": "theirs"
            }
        ]
    })
    .to_string();
    let (status, body) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/resolve-conflicts", card_id),
        Some(resolve_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "resolve conflicts failed: {}", body);

    let resolved_file =
        std::fs::read_to_string(Path::new(&repo_path).join("file.txt")).expect("resolved file should be readable");
    assert!(resolved_file.contains("modified-by-branch"));
    assert!(!resolved_file.contains("modified-by-main"));
}

#[tokio::test]
async fn test_resolve_conflicts_endpoint_manual_choice() {
    let (app, token, pool) = test_app_with_pool().await;
    let (_tmp, repo_path, branch_name) = create_test_repo_with_conflict();
    let card_id = seed_merge_ready_card(&pool, &repo_path, &branch_name).await;

    let _merge_result = start_conflicting_merge(app.clone(), &token, &card_id).await;

    let manual_content = "line1\nmanual-resolution\nline3\n";
    let resolve_body = json!({
        "resolutions": [
            {
                "file_path": "file.txt",
                "choice": "manual",
                "manual_content": manual_content
            }
        ]
    })
    .to_string();
    let (status, body) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/resolve-conflicts", card_id),
        Some(resolve_body),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "resolve conflicts failed: {}", body);

    let resolved_file =
        std::fs::read_to_string(Path::new(&repo_path).join("file.txt")).expect("resolved file should be readable");
    assert_eq!(resolved_file, manual_content);
}

#[tokio::test]
async fn test_complete_merge_fails_with_unresolved_conflicts() {
    let (app, token, pool) = test_app_with_pool().await;
    let (_tmp, repo_path, branch_name) = create_test_repo_with_conflict();
    let card_id = seed_merge_ready_card(&pool, &repo_path, &branch_name).await;

    let _merge_result = start_conflicting_merge(app.clone(), &token, &card_id).await;

    let (status, body) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/complete-merge", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let err: serde_json::Value =
        serde_json::from_str(&body).expect("error response should be valid JSON");
    let err_msg = err["error"].as_str().expect("error message should be present");
    assert!(err_msg.contains("conflicts remain"));

    let (status, body) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get card failed: {}", body);
    let card: serde_json::Value = serde_json::from_str(&body).expect("card response should be JSON");
    assert_eq!(card["stage"], "review");

    abort_merge_if_needed(&repo_path);
}

#[tokio::test]
async fn test_abort_merge_restores_clean_state() {
    let (app, token, pool) = test_app_with_pool().await;
    let (_tmp, repo_path, branch_name) = create_test_repo_with_conflict();
    let card_id = seed_merge_ready_card(&pool, &repo_path, &branch_name).await;

    let _merge_result = start_conflicting_merge(app.clone(), &token, &card_id).await;
    assert!(Path::new(&repo_path).join(".git/MERGE_HEAD").exists());

    let (status, body) = common::make_request(
        app.clone(),
        "POST",
        &format!("/api/cards/{}/abort-merge", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "abort merge failed: {}", body);
    assert!(!Path::new(&repo_path).join(".git/MERGE_HEAD").exists());
    assert!(!GitWorktreeService::is_merge_in_progress(&repo_path));

    let (status, body) = common::make_request(
        app,
        "GET",
        &format!("/api/cards/{}", card_id),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get card failed: {}", body);
    let card: serde_json::Value = serde_json::from_str(&body).expect("card response should be JSON");
    assert_eq!(card["stage"], "review");
}
