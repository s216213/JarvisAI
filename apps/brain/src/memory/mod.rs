use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Serialize, FromRow)]
pub struct UserFact {
    pub id: Uuid,
    pub key: String,
    pub value: String,
    pub source_thread: Option<Uuid>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserProfileView {
    pub display_name: Option<String>,
    pub preferences: serde_json::Value,
    pub response_style: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertFact {
    pub key: String,
    pub value: String,
    pub source_thread: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfile {
    pub display_name: Option<String>,
    pub preferences: Option<serde_json::Value>,
    pub response_style: Option<String>,
}

pub fn memory_router() -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_profile).put(update_profile))
        .route("/facts", get(list_facts).put(upsert_fact))
        .route("/facts/{id}", delete(delete_fact))
}

async fn get_profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<UserProfileView>> {
    let row = sqlx::query_as::<_, (Option<String>, serde_json::Value, Option<String>, Option<String>)>(
        r#"
        SELECT u.display_name, COALESCE(p.preferences, '{}'), p.response_style, u.display_name
        FROM users u
        LEFT JOIN user_profiles p ON p.user_id = u.id
        WHERE u.id = $1
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(UserProfileView {
        display_name: row.0,
        preferences: row.1,
        response_style: row.2,
    }))
}

async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpdateProfile>,
) -> AppResult<Json<UserProfileView>> {
    let now = chrono::Utc::now();
    if let Some(name) = &body.display_name {
        sqlx::query("UPDATE users SET display_name = $1, updated_at = $3 WHERE id = $2")
            .bind(name)
            .bind(user.id)
            .bind(now)
            .execute(&state.pool)
            .await?;
    }

    let prefs = body
        .preferences
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    sqlx::query(
        r#"
        INSERT INTO user_profiles (user_id, preferences, response_style, updated_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id) DO UPDATE SET
            preferences = COALESCE($2, user_profiles.preferences),
            response_style = COALESCE($3, user_profiles.response_style),
            updated_at = $4
        "#,
    )
    .bind(user.id)
    .bind(&prefs)
    .bind(&body.response_style)
    .bind(now)
    .execute(&state.pool)
    .await?;

    get_profile(State(state), user).await
}

async fn list_facts(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<UserFact>>> {
    let facts = sqlx::query_as::<_, UserFact>(
        r#"
        SELECT id, key, value, source_thread, updated_at
        FROM user_facts WHERE user_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(facts))
}

async fn upsert_fact(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<UpsertFact>,
) -> AppResult<Json<UserFact>> {
    if body.key.trim().is_empty() {
        return Err(AppError::BadRequest("key required".into()));
    }
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    sqlx::query(
        r#"
        INSERT INTO user_facts (id, user_id, key, value, source_thread, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (user_id, key) DO UPDATE SET
            value = EXCLUDED.value,
            source_thread = COALESCE(EXCLUDED.source_thread, user_facts.source_thread),
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(id)
    .bind(user.id)
    .bind(body.key.trim())
    .bind(&body.value)
    .bind(body.source_thread)
    .bind(now)
    .execute(&state.pool)
    .await?;

    let fact = sqlx::query_as::<_, UserFact>(
        r#"
        SELECT id, key, value, source_thread, updated_at
        FROM user_facts WHERE user_id = $1 AND key = $2
        "#,
    )
    .bind(user.id)
    .bind(body.key.trim())
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(fact))
}

async fn delete_fact(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<StatusOk>> {
    let res = sqlx::query("DELETE FROM user_facts WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("fact".into()));
    }
    Ok(Json(StatusOk { ok: true }))
}

#[derive(Serialize)]
struct StatusOk {
    ok: bool,
}

pub async fn build_memory_context(state: &AppState, user_id: Uuid, thread_id: Uuid) -> AppResult<String> {
    let profile = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        r#"
        SELECT u.display_name, p.response_style
        FROM users u
        LEFT JOIN user_profiles p ON p.user_id = u.id
        WHERE u.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    let facts = sqlx::query_as::<_, (String, String)>(
        r#"SELECT key, value FROM user_facts WHERE user_id = $1 ORDER BY updated_at DESC LIMIT 20"#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    let summary: Option<String> =
        sqlx::query_scalar("SELECT summary FROM thread_summaries WHERE thread_id = $1")
            .bind(thread_id)
            .fetch_optional(&state.pool)
            .await?;

    let mut parts = Vec::new();
    if let Some(name) = profile.0 {
        parts.push(format!("User display name: {name}"));
    }
    if let Some(style) = profile.1 {
        parts.push(format!("Preferred response style: {style}"));
    }
    if !facts.is_empty() {
        parts.push("Known facts:".into());
        for (k, v) in facts {
            parts.push(format!("- {k}: {v}"));
        }
    }
    if let Some(summary) = summary {
        parts.push(format!("Thread summary so far: {summary}"));
    }
    Ok(parts.join("\n"))
}

pub async fn maybe_update_summary(state: &AppState, thread_id: Uuid) -> AppResult<()> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE thread_id = $1")
            .bind(thread_id)
            .fetch_one(&state.pool)
            .await?;

    if count < 12 || count % 10 != 0 {
        return Ok(());
    }

    let recent = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT role, content FROM messages
        WHERE thread_id = $1
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .bind(thread_id)
    .fetch_all(&state.pool)
    .await?;

    let lines: Vec<String> = recent
        .into_iter()
        .rev()
        .map(|(role, content)| format!("{role}: {content}"))
        .collect();
    // Cheap extractive summary without an extra LLM call in MVP.
    let summary = if lines.len() > 8 {
        let head: Vec<_> = lines.iter().take(4).cloned().collect();
        let tail: Vec<_> = lines.iter().rev().take(4).cloned().collect::<Vec<_>>().into_iter().rev().collect();
        head.into_iter().chain(tail).collect::<Vec<_>>().join("\n")
    } else {
        lines.join("\n")
    };
    let last_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM messages WHERE thread_id = $1 ORDER BY created_at DESC LIMIT 1")
            .bind(thread_id)
            .fetch_optional(&state.pool)
            .await?;

    let now = chrono::Utc::now();
    sqlx::query(
        r#"
        INSERT INTO thread_summaries (thread_id, summary, up_to_message_id, updated_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (thread_id) DO UPDATE SET
            summary = EXCLUDED.summary,
            up_to_message_id = EXCLUDED.up_to_message_id,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(thread_id)
    .bind(summary)
    .bind(last_id)
    .bind(now)
    .execute(&state.pool)
    .await?;

    Ok(())
}
