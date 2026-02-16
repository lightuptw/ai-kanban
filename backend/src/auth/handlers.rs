use axum::{
    extract::{Extension, Multipart, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
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

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub nickname: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
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
    pub avatar_url: Option<String>,
    pub profile_completed: bool,
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

fn build_user_response(row: UserRow) -> UserResponse {
    let profile_completed = !row.nickname.is_empty() && (!row.email.is_empty() || !row.first_name.is_empty());
    let avatar_url = if row.has_avatar {
        Some(format!("/api/users/{}/avatar", row.id))
    } else {
        None
    };

    UserResponse {
        id: row.id,
        username: row.username,
        nickname: row.nickname,
        first_name: row.first_name,
        last_name: row.last_name,
        email: row.email,
        tenant_id: row.tenant_id,
        avatar_url,
        profile_completed,
    }
}

async fn fetch_user_row_by_id(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, username, nickname, first_name, last_name, email, tenant_id, (avatar IS NOT NULL AND length(avatar) > 0) as has_avatar FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
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
        user: build_user_response(UserRow {
            id: user_id,
            username,
            nickname,
            first_name,
            last_name,
            email,
            tenant_id,
            has_avatar: false,
        }),
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

    let user_id = user.id.clone();
    let tenant_id = user.tenant_id.clone();
    let has_avatar: bool = sqlx::query_scalar(
        "SELECT (avatar IS NOT NULL AND length(avatar) > 0) as has_avatar FROM users WHERE id = ?",
    )
    .bind(&user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| KanbanError::Unauthorized("Invalid username or password".into()))?;

    let user_response = build_user_response(UserRow {
        id: user_id.clone(),
        username: user.username,
        nickname: user.nickname,
        first_name: user.first_name,
        last_name: user.last_name,
        email: user.email,
        tenant_id: tenant_id.clone(),
        has_avatar,
    });

    let (token, refresh_token) = issue_tokens(db, &user_id, &tenant_id).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        user: user_response,
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

    let user = fetch_user_row_by_id(db, &refresh_token.user_id)
        .await?
        .map(build_user_response)
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

    let user = fetch_user_row_by_id(db, &auth_user.user_id)
        .await?
        .map(build_user_response)
        .ok_or_else(|| KanbanError::NotFound("User not found".into()))?;

    Ok(Json(user))
}

pub async fn update_profile(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, KanbanError> {
    let nickname = req.nickname.map(|value| value.trim().to_string());
    if let Some(ref value) = nickname {
        if value.is_empty() {
            return Err(KanbanError::BadRequest("Nickname cannot be empty".into()));
        }
    }

    let first_name = req.first_name.map(|value| value.trim().to_string());
    let last_name = req.last_name.map(|value| value.trim().to_string());
    let email = req.email.map(|value| value.trim().to_string());
    if let Some(ref value) = email {
        if !value.is_empty() && !value.contains('@') {
            return Err(KanbanError::BadRequest("Email must contain '@'".into()));
        }
    }

    let db = state.require_db()?;
    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        "UPDATE users SET nickname = COALESCE(?, nickname), first_name = COALESCE(?, first_name), last_name = COALESCE(?, last_name), email = COALESCE(?, email), updated_at = ? WHERE id = ?",
    )
    .bind(nickname)
    .bind(first_name)
    .bind(last_name)
    .bind(email)
    .bind(now)
    .bind(&auth_user.user_id)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(KanbanError::NotFound("User not found".into()));
    }

    let user = fetch_user_row_by_id(db, &auth_user.user_id)
        .await?
        .map(build_user_response)
        .ok_or_else(|| KanbanError::NotFound("User not found".into()))?;

    Ok(Json(user))
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    mut multipart: Multipart,
) -> Result<Json<UserResponse>, KanbanError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| KanbanError::BadRequest(format!("Failed to read multipart field: {}", e)))?
        .ok_or_else(|| KanbanError::BadRequest("No avatar file provided".into()))?;

    let content_type = field
        .content_type()
        .ok_or_else(|| KanbanError::BadRequest("Avatar content type is required".into()))?;
    if !matches!(content_type, "image/jpeg" | "image/png" | "image/webp") {
        return Err(KanbanError::BadRequest(
            "Avatar must be image/jpeg, image/png, or image/webp".into(),
        ));
    }

    let avatar_data = field
        .bytes()
        .await
        .map_err(|e| KanbanError::BadRequest(format!("Failed to read avatar data: {}", e)))?
        .to_vec();
    if avatar_data.len() > 2 * 1024 * 1024 {
        return Err(KanbanError::BadRequest(
            "Avatar must be 2MB or smaller".into(),
        ));
    }

    let db = state.require_db()?;
    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query("UPDATE users SET avatar = ?, updated_at = ? WHERE id = ?")
        .bind(avatar_data)
        .bind(now)
        .bind(&auth_user.user_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(KanbanError::NotFound("User not found".into()));
    }

    let user = fetch_user_row_by_id(db, &auth_user.user_id)
        .await?
        .map(build_user_response)
        .ok_or_else(|| KanbanError::NotFound("User not found".into()))?;

    Ok(Json(user))
}

pub async fn get_user_avatar(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, KanbanError> {
    let db = state.require_db()?;

    let avatar_row: Option<(Option<Vec<u8>>,)> = sqlx::query_as("SELECT avatar FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(db)
        .await?;

    let avatar_data = match avatar_row {
        Some((Some(data),)) if !data.is_empty() => data,
        _ => return Err(KanbanError::NotFound("Avatar not found".into())),
    };

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        avatar_data,
    ))
}
