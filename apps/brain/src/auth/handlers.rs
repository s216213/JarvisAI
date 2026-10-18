use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::{StatusCode, header, request::Parts},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::{hash_password, issue_access_token, issue_ws_ticket, verify_password, verify_token},
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub role: String,
}

#[derive(Clone, Debug)]
pub struct RequireSuperadmin(pub AuthUser);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let auth = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        let token = auth
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;
        let claims = verify_token(&state.config.jwt_secret, token)?;
        if claims.kind != crate::auth::TokenKind::Access
            && claims.kind != crate::auth::TokenKind::WsTicket
        {
            return Err(AppError::Unauthorized);
        }
        Ok(AuthUser {
            id: claims.sub,
            email: claims.email,
            role: claims.role,
        })
    }
}

impl FromRequestParts<AppState> for RequireSuperadmin {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != "superadmin" {
            return Err(AppError::Forbidden);
        }
        Ok(RequireSuperadmin(user))
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct AcceptInviteRequest {
    pub token: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserPublic,
}

#[derive(Serialize)]
pub struct UserPublic {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub display_name: Option<String>,
}

#[derive(Serialize)]
pub struct WsTicketResponse {
    pub ticket: String,
    pub expires_in: i64,
}

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/invite/accept", post(accept_invite))
        .route("/ws-ticket", post(ws_ticket))
        .route("/me", get(me))
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<TokenPair>> {
    let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, String)>(
        r#"SELECT id, email, role, display_name, password_hash FROM users WHERE email = $1"#,
    )
    .bind(body.email.to_lowercase())
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let (id, email, role, display_name, password_hash) = row;
    if !verify_password(&body.password, &password_hash)? {
        return Err(AppError::Unauthorized);
    }

    issue_pair(&state, id, &email, &role, display_name).await
}

async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> AppResult<Json<TokenPair>> {
    let hash = hash_token(&body.refresh_token);
    let now = Utc::now();
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, Option<String>)>(
        r#"
        SELECT rt.id, u.id, u.email, u.role, u.display_name
        FROM refresh_tokens rt
        JOIN users u ON u.id = rt.user_id
        WHERE rt.token_hash = $1
          AND rt.revoked_at IS NULL
          AND rt.expires_at > $2
        "#,
    )
    .bind(&hash)
    .bind(now)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let (_rt_id, user_id, email, role, display_name) = row;
    sqlx::query("UPDATE refresh_tokens SET revoked_at = $2 WHERE token_hash = $1")
        .bind(&hash)
        .bind(now)
        .execute(&state.pool)
        .await?;

    issue_pair(&state, user_id, &email, &role, display_name).await
}

async fn accept_invite(
    State(state): State<AppState>,
    Json(body): Json<AcceptInviteRequest>,
) -> AppResult<(StatusCode, Json<TokenPair>)> {
    let token_hash = hash_token(&body.token);
    let now = Utc::now();
    let invite = sqlx::query_as::<_, (Uuid, String, String)>(
        r#"
        SELECT id, email, role FROM invites
        WHERE token_hash = $1 AND accepted_at IS NULL AND expires_at > $2
        "#,
    )
    .bind(&token_hash)
    .bind(now)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("invalid or expired invite".into()))?;

    let (invite_id, email, role) = invite;
    if body.password.len() < 8 {
        return Err(AppError::BadRequest("password must be at least 8 characters".into()));
    }

    let password_hash = hash_password(&body.password)?;
    let mut tx = state.pool.begin().await?;
    let user_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, role, display_name)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(user_id)
    .bind(email.to_lowercase())
    .bind(&password_hash)
    .bind(&role)
    .bind(&body.display_name)
    .execute(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.constraint().is_some() => {
            AppError::Conflict("user already exists".into())
        }
        other => AppError::Db(other),
    })?;

    sqlx::query("INSERT INTO user_profiles (user_id) VALUES ($1)")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE invites SET accepted_at = $2 WHERE id = $1")
        .bind(invite_id)
        .bind(now)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let pair = issue_pair(&state, user_id, &email, &role, body.display_name).await?;
    Ok((StatusCode::CREATED, pair))
}

async fn ws_ticket(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<WsTicketResponse>> {
    let ticket = issue_ws_ticket(
        &state.config.jwt_secret,
        user.id,
        &user.email,
        &user.role,
        state.config.ws_ticket_ttl_secs,
    )?;
    Ok(Json(WsTicketResponse {
        ticket,
        expires_in: state.config.ws_ticket_ttl_secs,
    }))
}

async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<UserPublic>> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"SELECT id, email, role, display_name FROM users WHERE id = $1"#,
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(UserPublic {
        id: row.id,
        email: row.email,
        role: row.role,
        display_name: row.display_name,
    }))
}

async fn issue_pair(
    state: &AppState,
    user_id: Uuid,
    email: &str,
    role: &str,
    display_name: Option<String>,
) -> AppResult<Json<TokenPair>> {
    let access_token = issue_access_token(
        &state.config.jwt_secret,
        user_id,
        email,
        role,
        state.config.access_token_ttl_secs,
    )?;
    let refresh_token = Uuid::new_v4().to_string() + &Uuid::new_v4().to_string();
    let refresh_hash = hash_token(&refresh_token);
    let expires_at = Utc::now() + Duration::seconds(state.config.refresh_token_ttl_secs);

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&refresh_hash)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    Ok(Json(TokenPair {
        access_token,
        refresh_token,
        user: UserPublic {
            id: user_id,
            email: email.to_string(),
            role: role.to_string(),
            display_name,
        },
    }))
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn bootstrap_superadmin(state: &AppState) -> anyhow::Result<()> {
    let Some(email) = &state.config.bootstrap_superadmin_email else {
        return Ok(());
    };
    let Some(password) = &state.config.bootstrap_superadmin_password else {
        return Ok(());
    };

    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE role = 'superadmin')")
        .fetch_one(&state.pool)
        .await?;
    if exists {
        return Ok(());
    }

    let password_hash = hash_password(password)?;
    let user_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, role, display_name)
        VALUES ($1, $2, $3, 'superadmin', 'Superadmin')
        "#,
    )
    .bind(user_id)
    .bind(email.to_lowercase())
    .bind(password_hash)
    .execute(&state.pool)
    .await?;

    sqlx::query("INSERT INTO user_profiles (user_id) VALUES ($1)")
        .bind(user_id)
        .execute(&state.pool)
        .await?;

    tracing::info!(%email, "bootstrapped superadmin");
    Ok(())
}
