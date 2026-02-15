use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::state::AppState;
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

fn extract_token(req: &Request) -> Option<String> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_string);

    auth_header.or_else(|| {
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
