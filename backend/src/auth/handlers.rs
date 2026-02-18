use axum::{
    body::Body,
    extract::{Extension, Multipart, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::api::state::AppState;
use crate::auth::{avatar, cookies, jwt, middleware::AuthUser, password};
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

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
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

impl From<UserRow> for UserResponse {
    fn from(row: UserRow) -> Self {
        build_user_response(row)
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

fn build_user_response(row: UserRow) -> UserResponse {
    let profile_completed = !row.nickname.is_empty() && (!row.email.is_empty() || !row.first_name.is_empty());
    let avatar_url = if row.has_avatar {
        Some(format!("/api/auth/avatar/{}", row.username))
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
        has_avatar: row.has_avatar,
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
) -> Result<Response, KanbanError> {
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
    let secure = state.config.cookie_secure;

    build_auth_response(
        build_user_response(UserRow {
            id: user_id,
            username,
            nickname,
            first_name,
            last_name,
            email,
            tenant_id,
            has_avatar: false,
        }),
        &token,
        &refresh_token,
        secure,
    )
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Response, KanbanError> {
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
    let secure = state.config.cookie_secure;

    build_auth_response(
        user_response,
        &token,
        &refresh_token,
        secure,
    )
}

pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: Option<Json<RefreshRequest>>,
) -> Result<Response, KanbanError> {
    let db = state.require_db()?;
    let refresh_token = extract_cookie_value(&headers, cookies::REFRESH_TOKEN_COOKIE)
        .or_else(|| req.map(|Json(body)| body.refresh_token.trim().to_string()))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| KanbanError::Unauthorized("Invalid refresh token".into()))?;
    let token_hash = jwt::hash_refresh_token(&refresh_token);
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
    let secure = state.config.cookie_secure;

    build_auth_response(user, &token, &refresh_token, secure)
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, KanbanError> {
    let db = state.require_db()?;

    if let Some(refresh_token) = extract_cookie_value(&headers, cookies::REFRESH_TOKEN_COOKIE) {
        let token_hash = jwt::hash_refresh_token(refresh_token.trim());

        sqlx::query("UPDATE refresh_tokens SET revoked = 1 WHERE token_hash = ?")
            .bind(token_hash)
            .execute(db)
            .await?;
    }

    let secure = state.config.cookie_secure;
    let clear_access_cookie = cookies::build_clear_cookie(cookies::ACCESS_TOKEN_COOKIE, secure);
    let clear_refresh_cookie = cookies::build_clear_cookie(cookies::REFRESH_TOKEN_COOKIE, secure);

    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(header::SET_COOKIE, clear_access_cookie)
        .header(header::SET_COOKIE, clear_refresh_cookie)
        .body(Body::empty())
        .map_err(|e| KanbanError::Internal(format!("Failed to build auth response: {}", e)))
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

fn build_auth_response(
    user: UserResponse,
    access_token: &str,
    refresh_token: &str,
    secure: bool,
) -> Result<Response, KanbanError> {
    let access_cookie = cookies::build_token_cookie(
        cookies::ACCESS_TOKEN_COOKIE,
        access_token,
        15 * 60,
        secure,
    );
    let refresh_cookie = cookies::build_token_cookie(
        cookies::REFRESH_TOKEN_COOKIE,
        refresh_token,
        30 * 24 * 60 * 60,
        secure,
    );
    let response_body = Json(LoginResponse { user }).into_response().into_body();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::SET_COOKIE, access_cookie)
        .header(header::SET_COOKIE, refresh_cookie)
        .body(response_body)
        .map_err(|e| KanbanError::Internal(format!("Failed to build auth response: {}", e)))
}

pub(crate) fn extract_cookie_value(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header.split(';').find_map(|cookie| {
                let mut split = cookie.trim().splitn(2, '=');
                let name = split.next()?.trim();
                let value = split.next()?.trim();

                if name == cookie_name && !value.is_empty() {
                    Some(value.to_string())
                } else {
                    None
                }
            })
        })
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

pub async fn change_password(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, KanbanError> {
    if req.new_password.len() < 8 {
        return Err(KanbanError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }

    let db = state.require_db()?;
    let password_hash: Option<(String,)> =
        sqlx::query_as("SELECT password_hash FROM users WHERE id = ?")
            .bind(&auth_user.user_id)
            .fetch_optional(db)
            .await?;

    let current_hash = password_hash
        .map(|(hash,)| hash)
        .ok_or_else(|| KanbanError::NotFound("User not found".into()))?;

    let valid = password::verify_password(&req.current_password, &current_hash)
        .map_err(|e| KanbanError::Internal(format!("Failed to verify password: {}", e)))?;
    if !valid {
        return Err(KanbanError::Unauthorized("Current password is incorrect".into()));
    }

    let new_hash = password::hash_password(&req.new_password)
        .map_err(|e| KanbanError::Internal(format!("Failed to hash password: {}", e)))?;
    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
        .bind(new_hash)
        .bind(now)
        .bind(&auth_user.user_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(KanbanError::NotFound("User not found".into()));
    }

    Ok(Json(serde_json::json!({
        "message": "Password changed successfully"
    })))
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
