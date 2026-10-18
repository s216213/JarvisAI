use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::{RequireSuperadmin, hash_token},
    error::{AppError, AppResult},
    llm::router::guardian_snapshot,
    state::AppState,
    tools::CATALOG,
};

#[derive(Debug, Serialize, FromRow)]
pub struct UserAdminRow {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub display_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvite {
    pub email: String,
    pub role: Option<String>,
    pub expires_hours: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct InviteCreated {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AuditRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub detail: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/invites", post(create_invite))
        .route("/providers", get(provider_health))
        .route("/tools", get(list_tools))
        .route("/audit", get(list_audit))
        .route("/devices", get(list_devices_admin))
}

async fn list_users(
    State(state): State<AppState>,
    _admin: RequireSuperadmin,
) -> AppResult<Json<Vec<UserAdminRow>>> {
    let rows = sqlx::query_as::<_, UserAdminRow>(
        r#"
        SELECT id, email, role, display_name, created_at
        FROM users ORDER BY created_at ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

async fn create_invite(
    State(state): State<AppState>,
    admin: RequireSuperadmin,
    Json(body): Json<CreateInvite>,
) -> AppResult<Json<InviteCreated>> {
    let role = body.role.unwrap_or_else(|| "user".into());
    if role != "user" && role != "superadmin" {
        return Err(AppError::BadRequest("role must be user or superadmin".into()));
    }
    let hours = body.expires_hours.unwrap_or(72);
    let token: String = rand::rng()
        .sample_iter(Alphanumeric)
        .take(40)
        .map(char::from)
        .collect();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::hours(hours);

    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO invites (id, email, token_hash, role, created_by, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id)
    .bind(body.email.to_lowercase())
    .bind(&token_hash)
    .bind(&role)
    .bind(admin.0.id)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        r#"INSERT INTO audit_logs (id, user_id, action, detail) VALUES ($1, $2, 'invite.create', $3)"#,
    )
    .bind(Uuid::new_v4())
    .bind(admin.0.id)
    .bind(serde_json::json!({ "invite_id": id, "email": body.email, "role": role }))
    .execute(&state.pool)
    .await?;

    Ok(Json(InviteCreated {
        id,
        email: body.email.to_lowercase(),
        role,
        token,
        expires_at,
    }))
}

async fn provider_health(
    State(state): State<AppState>,
    _admin: RequireSuperadmin,
) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "providers": guardian_snapshot(&state.quota),
        "configured": {
            "groq": state.config.groq_api_key.is_some(),
            "gemini": state.config.gemini_api_key.is_some(),
            "openrouter": state.config.openrouter_api_key.is_some(),
        }
    })))
}

async fn list_tools(
    _admin: RequireSuperadmin,
) -> AppResult<Json<serde_json::Value>> {
    let tools: Vec<_> = CATALOG
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "capability": t.capability,
                "trust": t.trust,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "tools": tools })))
}

async fn list_audit(
    State(state): State<AppState>,
    _admin: RequireSuperadmin,
) -> AppResult<Json<Vec<AuditRow>>> {
    let rows = sqlx::query_as::<_, AuditRow>(
        r#"
        SELECT id, user_id, action, detail, created_at
        FROM audit_logs
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

async fn list_devices_admin(
    State(state): State<AppState>,
    admin: RequireSuperadmin,
) -> AppResult<Json<serde_json::Value>> {
    // Superadmin sees their own live devices + DB snapshot
    let live = state.devices.list_for_user(admin.0.id);
    let db = sqlx::query_as::<_, (String, String, Option<String>, serde_json::Value, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT device_id, platform, label, capabilities, last_seen
        FROM devices
        ORDER BY last_seen DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(serde_json::json!({
        "live": live,
        "persisted": db.into_iter().map(|(id, platform, label, caps, last_seen)| {
            serde_json::json!({
                "device_id": id,
                "platform": platform,
                "label": label,
                "capabilities": caps,
                "last_seen": last_seen,
            })
        }).collect::<Vec<_>>()
    })))
}
