use std::collections::HashSet;

use anyhow::Result;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub tid: String,
    pub exp: usize,
    pub iat: usize,
    pub nbf: usize,
    pub jti: String,
}

pub async fn get_or_create_signing_key(pool: &SqlitePool) -> Result<Vec<u8>> {
    let existing: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT value FROM app_secrets WHERE key = ?")
            .bind("jwt_signing_key")
            .fetch_optional(pool)
            .await?;

    if let Some((value,)) = existing {
        return Ok(value);
    }

    let mut key_bytes = [0_u8; 64];
    rand::thread_rng().fill_bytes(&mut key_bytes);
    let key = key_bytes.to_vec();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query("INSERT OR IGNORE INTO app_secrets (key, value, created_at) VALUES (?, ?, ?)")
        .bind("jwt_signing_key")
        .bind(&key)
        .bind(&now)
        .execute(pool)
        .await?;

    let stored: (Vec<u8>,) = sqlx::query_as("SELECT value FROM app_secrets WHERE key = ?")
        .bind("jwt_signing_key")
        .fetch_one(pool)
        .await?;

    Ok(stored.0)
}

pub fn create_token(key: &[u8], user_id: &str, tenant_id: &str) -> Result<String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        tid: tenant_id.to_string(),
        exp: now + 15 * 60,
        iat: now,
        nbf: now,
        jti: Uuid::new_v4().to_string(),
    };

    let token = encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(key))?;
    Ok(token)
}

pub fn create_refresh_token() -> String {
    let mut token_bytes = [0_u8; 64];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    token_bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn verify_token(key: &[u8], token: &str) -> Result<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_nbf = true;
    validation.required_spec_claims = HashSet::from([
        "exp".to_string(),
        "nbf".to_string(),
        "iat".to_string(),
        "sub".to_string(),
        "tid".to_string(),
        "jti".to_string(),
    ]);

    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(key), &validation)?;
    Ok(token_data.claims)
}
