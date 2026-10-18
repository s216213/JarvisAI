use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::RequireSuperadmin,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Serialize, FromRow)]
pub struct PersonaVersion {
    pub id: Uuid,
    pub label: String,
    pub system_prompt: String,
    pub created_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreatePersona {
    pub label: String,
    pub system_prompt: String,
    pub activate: Option<bool>,
}

pub fn persona_router() -> Router<AppState> {
    Router::new()
        .route("/active", get(get_active))
        .route("/", get(list_versions).post(create_version))
        .route("/{id}/activate", post(activate_version))
}

async fn get_active(State(state): State<AppState>) -> AppResult<Json<PersonaVersion>> {
    let row = sqlx::query_as::<_, PersonaVersion>(
        r#"
        SELECT id, label, system_prompt, created_by, created_at, is_active
        FROM persona_versions WHERE is_active = true LIMIT 1
        "#,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("active persona".into()))?;
    Ok(Json(row))
}

async fn list_versions(
    State(state): State<AppState>,
    _admin: RequireSuperadmin,
) -> AppResult<Json<Vec<PersonaVersion>>> {
    let rows = sqlx::query_as::<_, PersonaVersion>(
        r#"
        SELECT id, label, system_prompt, created_by, created_at, is_active
        FROM persona_versions
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

async fn create_version(
    State(state): State<AppState>,
    admin: RequireSuperadmin,
    Json(body): Json<CreatePersona>,
) -> AppResult<Json<PersonaVersion>> {
    if body.label.trim().is_empty() || body.system_prompt.trim().is_empty() {
        return Err(AppError::BadRequest("label and system_prompt required".into()));
    }

    let mut tx = state.pool.begin().await?;
    let activate = body.activate.unwrap_or(true);
    if activate {
        sqlx::query("UPDATE persona_versions SET is_active = false WHERE is_active = true")
            .execute(&mut *tx)
            .await?;
    }

    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    sqlx::query(
        r#"
        INSERT INTO persona_versions (id, label, system_prompt, created_by, created_at, is_active)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id)
    .bind(body.label.trim())
    .bind(&body.system_prompt)
    .bind(admin.0.id)
    .bind(now)
    .bind(activate)
    .execute(&mut *tx)
    .await?;

    let row = sqlx::query_as::<_, PersonaVersion>(
        r#"
        SELECT id, label, system_prompt, created_by, created_at, is_active
        FROM persona_versions WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO audit_logs (id, user_id, action, detail) VALUES ($1, $2, 'persona.create', $3)"#,
    )
    .bind(Uuid::new_v4())
    .bind(admin.0.id)
    .bind(serde_json::json!({ "id": row.id, "label": row.label, "activated": activate }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(row))
}

async fn activate_version(
    State(state): State<AppState>,
    admin: RequireSuperadmin,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PersonaVersion>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE persona_versions SET is_active = false WHERE is_active = true")
        .execute(&mut *tx)
        .await?;

    let row = sqlx::query_as::<_, PersonaVersion>(
        r#"
        UPDATE persona_versions SET is_active = true WHERE id = $1
        RETURNING id, label, system_prompt, created_by, created_at, is_active
        "#,
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("persona version".into()))?;

    sqlx::query(
        r#"INSERT INTO audit_logs (id, user_id, action, detail) VALUES ($1, $2, 'persona.activate', $3)"#,
    )
    .bind(Uuid::new_v4())
    .bind(admin.0.id)
    .bind(serde_json::json!({ "id": row.id, "label": row.label }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(row))
}

pub async fn load_active_prompt(state: &AppState) -> AppResult<String> {
    let prompt: Option<String> =
        sqlx::query_scalar("SELECT system_prompt FROM persona_versions WHERE is_active = true LIMIT 1")
            .fetch_optional(&state.pool)
            .await?;
    Ok(prompt.unwrap_or_else(|| {
        "You are Jarvis, a precise personal AI assistant.".to_string()
    }))
}
