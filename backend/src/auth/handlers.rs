use axum::{
    extract::{Extension, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::state::AppState;
use crate::auth::{jwt, middleware::AuthUser, password};
use crate::domain::KanbanError;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub nickname: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub tenant_id: String,
}

#[derive(Debug, sqlx::FromRow)]
struct UserWithPassword {
    pub id: String,
    pub tenant_id: String,
    pub username: String,
    pub nickname: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug, sqlx::FromRow)]
struct RefreshTokenRow {
    pub id: String,
    pub user_id: String,
}

async fn issue_tokens(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    tenant_id: &str,
) -> Result<(String, String), KanbanError> {
    let signing_key = jwt::get_or_create_signing_key(pool)
        .await
        .map_err(|e| KanbanError::Internal(format!("Failed to load JWT signing key: {}", e)))?;
    let token = jwt::create_token(&signing_key, user_id, tenant_id)
        .map_err(|e| KanbanError::Internal(format!("Failed to create JWT token: {}", e)))?;

    let refresh_token = jwt::create_refresh_token();
    let refresh_token_hash = jwt::hash_refresh_token(&refresh_token);
    let now = chrono::Utc::now();
    let created_at = now.to_rfc3339();
    let expires_at = (now + chrono::Duration::days(30)).to_rfc3339();

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at, revoked) VALUES (?, ?, ?, ?, ?, 0)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(refresh_token_hash)
    .bind(expires_at)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok((token, refresh_token))
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, KanbanError> {
    let username = req.username.trim().to_string();
    let password = req.password;
    let nickname = req.nickname.trim().to_string();
    let first_name = req.first_name.unwrap_or_default();
    let last_name = req.last_name.unwrap_or_default();
    let email = req.email.unwrap_or_default();

    if username.len() < 3 {
        return Err(KanbanError::BadRequest(
            "Username must be at least 3 characters".into(),
        ));
    }
    if password.len() < 8 {
        return Err(KanbanError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }
    if nickname.is_empty() {
        return Err(KanbanError::BadRequest("Nickname is required".into()));
    }

    let db = state.require_db()?;

    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE username = ?")
        .bind(&username)
        .fetch_optional(db)
        .await?;

    if existing.is_some() {
        return Err(KanbanError::BadRequest("Username is already taken".into()));
    }

    let password_hash = password::hash_password(&password)
        .map_err(|e| KanbanError::Internal(format!("Failed to hash password: {}", e)))?;

    let user_id = Uuid::new_v4().to_string();
    let tenant_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO users (id, tenant_id, username, nickname, first_name, last_name, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(&tenant_id)
    .bind(&username)
    .bind(&nickname)
    .bind(&first_name)
    .bind(&last_name)
    .bind(&email)
    .bind(&password_hash)
    .bind(&now)
    .bind(&now)
    .execute(db)
    .await?;

    let (token, refresh_token) = issue_tokens(db, &user_id, &tenant_id).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        user: UserResponse {
            id: user_id,
            username,
            nickname,
            first_name,
            last_name,
            email,
            tenant_id,
        },
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, KanbanError> {
    let username = req.username.trim().to_string();

    let db = state.require_db()?;

    let user: Option<UserWithPassword> = sqlx::query_as(
        "SELECT id, tenant_id, username, nickname, first_name, last_name, email, password_hash FROM users WHERE username = ?",
    )
    .bind(&username)
    .fetch_optional(db)
    .await?;

    let user = user.ok_or_else(|| KanbanError::Unauthorized("Invalid username or password".into()))?;

    let valid = password::verify_password(&req.password, &user.password_hash)
        .map_err(|e| KanbanError::Internal(format!("Failed to verify password: {}", e)))?;

    if !valid {
        return Err(KanbanError::Unauthorized(
            "Invalid username or password".into(),
        ));
    }

    let (token, refresh_token) = issue_tokens(db, &user.id, &user.tenant_id).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        user: UserResponse {
            id: user.id,
            username: user.username,
            nickname: user.nickname,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            tenant_id: user.tenant_id,
        },
    }))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, KanbanError> {
    let db = state.require_db()?;
    let token_hash = jwt::hash_refresh_token(req.refresh_token.trim());
    let now = chrono::Utc::now().to_rfc3339();

    let refresh_token: RefreshTokenRow = sqlx::query_as(
        "SELECT id, user_id FROM refresh_tokens WHERE token_hash = ? AND revoked = 0 AND expires_at > ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(token_hash)
    .bind(now)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| KanbanError::Unauthorized("Invalid refresh token".into()))?;

    sqlx::query("UPDATE refresh_tokens SET revoked = 1 WHERE id = ?")
        .bind(&refresh_token.id)
        .execute(db)
        .await?;

    let user: UserResponse = sqlx::query_as(
        "SELECT id, username, nickname, first_name, last_name, email, tenant_id FROM users WHERE id = ?",
    )
    .bind(&refresh_token.user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| KanbanError::Unauthorized("Invalid refresh token".into()))?;

    let (token, refresh_token) = issue_tokens(db, &user.id, &user.tenant_id).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        user,
    }))
}

pub async fn me(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<UserResponse>, KanbanError> {
    let db = state.require_db()?;

    let user: UserResponse = sqlx::query_as(
        "SELECT id, username, nickname, first_name, last_name, email, tenant_id FROM users WHERE id = ?",
    )
    .bind(&auth_user.user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| KanbanError::NotFound("User not found".into()))?;

    Ok(Json(user))
}
