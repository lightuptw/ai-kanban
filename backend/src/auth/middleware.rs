use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::state::AppState;
use crate::auth::cookies;
use crate::auth::handlers::extract_cookie_value;
use crate::auth::jwt;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub tenant_id: String,
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(service_key) = extract_service_key(&req) {
        let db = state.require_db().map_err(|_| StatusCode::UNAUTHORIZED)?;

        match req.extensions().get::<ConnectInfo<SocketAddr>>() {
            Some(connect_info) if !connect_info.0.ip().is_loopback() => {
                return Err(StatusCode::FORBIDDEN);
            }
            None => {
                tracing::warn!("Missing ConnectInfo; skipping localhost validation for service key");
            }
            _ => {}
        }

        let stored_key: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT value FROM app_secrets WHERE key = ?")
                .bind("service_api_key")
                .fetch_optional(db)
                .await
                .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let stored_key = stored_key.ok_or(StatusCode::UNAUTHORIZED)?;
        let stored_key = String::from_utf8(stored_key.0).map_err(|_| StatusCode::UNAUTHORIZED)?;
        if stored_key != service_key {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let service_user_id: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT value FROM app_secrets WHERE key = ?")
                .bind("service_user_id")
                .fetch_optional(db)
                .await
                .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let service_user_id = service_user_id.ok_or(StatusCode::UNAUTHORIZED)?;
        let service_user_id =
            String::from_utf8(service_user_id.0).map_err(|_| StatusCode::UNAUTHORIZED)?;

        let service_user: Option<(String,)> =
            sqlx::query_as("SELECT tenant_id FROM users WHERE id = ?")
                .bind(&service_user_id)
                .fetch_optional(db)
                .await
                .map_err(|_| StatusCode::UNAUTHORIZED)?;
        let (tenant_id,) = service_user.ok_or(StatusCode::UNAUTHORIZED)?;

        req.extensions_mut().insert(AuthUser {
            user_id: service_user_id,
            tenant_id,
        });

        return Ok(next.run(req).await);
    }

    let token = extract_token(&req).ok_or(StatusCode::UNAUTHORIZED)?;

    let db = state
        .require_db()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let signing_key = jwt::get_or_create_signing_key(db)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let claims = jwt::verify_token(&signing_key, &token).map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(AuthUser {
        user_id: claims.sub,
        tenant_id: claims.tid,
    });

    Ok(next.run(req).await)
}

fn extract_service_key(req: &Request) -> Option<String> {
    req.headers()
        .get("X-Service-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn extract_token(req: &Request) -> Option<String> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_string);

    auth_header
        .or_else(|| extract_cookie_value(req.headers(), cookies::ACCESS_TOKEN_COOKIE))
        .or_else(|| {
            req.uri().query().and_then(|query| {
                query
                    .split('&')
                    .filter_map(|part| {
                        let mut split = part.splitn(2, '=');
                        let key = split.next()?;
                        let value = split.next().unwrap_or_default();
                        Some((key, value))
                    })
                    .find(|(key, _)| *key == "token")
                    .map(|(_, value)| value.to_string())
            })
        })
}
