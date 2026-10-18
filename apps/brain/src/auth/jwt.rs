use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    Access,
    WsTicket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub role: String,
    pub kind: TokenKind,
    pub exp: i64,
    pub iat: i64,
}

pub fn issue_access_token(
    secret: &str,
    user_id: Uuid,
    email: &str,
    role: &str,
    ttl_secs: i64,
) -> AppResult<String> {
    issue(secret, user_id, email, role, TokenKind::Access, ttl_secs)
}

pub fn issue_ws_ticket(
    secret: &str,
    user_id: Uuid,
    email: &str,
    role: &str,
    ttl_secs: i64,
) -> AppResult<String> {
    issue(secret, user_id, email, role, TokenKind::WsTicket, ttl_secs)
}

fn issue(
    secret: &str,
    user_id: Uuid,
    email: &str,
    role: &str,
    kind: TokenKind,
    ttl_secs: i64,
) -> AppResult<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        kind,
        iat: now.timestamp(),
        exp: (now + Duration::seconds(ttl_secs)).timestamp(),
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

pub fn verify_token(secret: &str, token: &str) -> AppResult<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}
