use axum::{
    extract::{Extension, Multipart, Path, State},
    http::{header, HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::api::state::AppState;
use crate::auth::{avatar, jwt, middleware::AuthUser, password};
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

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub nickname: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub tenant_id: String,
    pub has_avatar: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct UserRow {
    pub id: String,
    pub username: String,
    pub nickname: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub tenant_id: String,
    pub has_avatar: bool,
}

impl From<UserRow> for UserResponse {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            username: row.username,
            nickname: row.nickname,
            first_name: row.first_name,
            last_name: row.last_name,
            email: row.email,
            tenant_id: row.tenant_id,
            has_avatar: row.has_avatar,
        }
    }
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
    pub has_avatar: bool,
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
            has_avatar: false,
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
        "SELECT id, tenant_id, username, nickname, first_name, last_name, email, password_hash, (avatar IS NOT NULL) as has_avatar FROM users WHERE username = ?",
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
            has_avatar: user.has_avatar,
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

    let user: UserRow = sqlx::query_as(
        "SELECT id, username, nickname, first_name, last_name, email, tenant_id, (avatar IS NOT NULL) as has_avatar FROM users WHERE id = ?",
    )
    .bind(&refresh_token.user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| KanbanError::Unauthorized("Invalid refresh token".into()))?;

    let (token, refresh_token) = issue_tokens(db, &user.id, &user.tenant_id).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        user: user.into(),
    }))
}

pub async fn me(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<UserResponse>, KanbanError> {
    let db = state.require_db()?;

    let user: UserRow = sqlx::query_as(
        "SELECT id, username, nickname, first_name, last_name, email, tenant_id, (avatar IS NOT NULL) as has_avatar FROM users WHERE id = ?",
    )
    .bind(&auth_user.user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| KanbanError::NotFound("User not found".into()))?;

    Ok(Json(user.into()))
}

const MAX_AVATAR_SIZE: usize = 2 * 1024 * 1024;

pub async fn upload_avatar(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    mut multipart: Multipart,
) -> Result<Json<UserResponse>, KanbanError> {
    let db = state.require_db()?;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| KanbanError::BadRequest(format!("Invalid multipart data: {}", e)))?
        .ok_or_else(|| KanbanError::BadRequest("No file provided".into()))?;

    let bytes = field
        .bytes()
        .await
        .map_err(|e| KanbanError::BadRequest(format!("Failed to read file: {}", e)))?;

    if bytes.len() > MAX_AVATAR_SIZE {
        return Err(KanbanError::BadRequest(format!(
            "File too large: {} bytes (max {} bytes)",
            bytes.len(),
            MAX_AVATAR_SIZE
        )));
    }

    let content_type = avatar::detect_content_type(&bytes).ok_or_else(|| {
        KanbanError::BadRequest(
            "Unsupported image format. Allowed: JPEG, PNG, WebP".into(),
        )
    })?;

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE users SET avatar = ?, avatar_content_type = ?, updated_at = ? WHERE id = ?")
        .bind(bytes.as_ref())
        .bind(content_type)
        .bind(&now)
        .bind(&auth_user.user_id)
        .execute(db)
        .await?;

    let user: UserRow = sqlx::query_as(
        "SELECT id, username, nickname, first_name, last_name, email, tenant_id, (avatar IS NOT NULL) as has_avatar FROM users WHERE id = ?",
    )
    .bind(&auth_user.user_id)
    .fetch_one(db)
    .await?;

    Ok(Json(user.into()))
}

pub async fn get_avatar(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> Result<axum::response::Response, KanbanError> {
    let db = state.require_db()?;

    let row: Option<(Vec<u8>, Option<String>)> = sqlx::query_as(
        "SELECT avatar, avatar_content_type FROM users WHERE id = ? AND avatar IS NOT NULL",
    )
    .bind(&user_id)
    .fetch_optional(db)
    .await?;

    let (avatar_bytes, stored_content_type) =
        row.ok_or_else(|| KanbanError::NotFound("No avatar found".into()))?;

    let content_type = stored_content_type
        .or_else(|| avatar::detect_content_type(&avatar_bytes).map(String::from))
        .unwrap_or_else(|| "application/octet-stream".into());

    let mut hasher = Sha256::new();
    hasher.update(&avatar_bytes);
    let etag = format!(
        "\"{}\"",
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );

    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
        if let Ok(val) = if_none_match.to_str() {
            if val == etag {
                return Ok(axum::response::Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .body(axum::body::Body::empty())
                    .unwrap());
            }
        }
    }

    Ok(axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .header(header::ETAG, etag)
        .body(axum::body::Body::from(avatar_bytes))
        .unwrap())
}

pub async fn delete_avatar(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<StatusCode, KanbanError> {
    let db = state.require_db()?;

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE users SET avatar = NULL, avatar_content_type = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&auth_user.user_id)
    .execute(db)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
