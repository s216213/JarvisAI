use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, put},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    auth::RequireSuperadmin,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Serialize, FromRow)]
pub struct FeatureFlag {
    pub key: String,
    pub enabled: bool,
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFlag {
    pub enabled: Option<bool>,
    pub value: Option<serde_json::Value>,
}

pub fn flags_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_flags))
        .route("/{key}", put(update_flag))
}

async fn list_flags(
    State(state): State<AppState>,
    _admin: RequireSuperadmin,
) -> AppResult<Json<Vec<FeatureFlag>>> {
    let rows = sqlx::query_as::<_, FeatureFlag>(
        "SELECT key, enabled, value FROM feature_flags ORDER BY key",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

async fn update_flag(
    State(state): State<AppState>,
    admin: RequireSuperadmin,
    Path(key): Path<String>,
    Json(body): Json<UpdateFlag>,
) -> AppResult<Json<FeatureFlag>> {
    let row = sqlx::query_as::<_, FeatureFlag>(
        r#"
        UPDATE feature_flags SET
            enabled = COALESCE($2, enabled),
            value = COALESCE($3, value),
            updated_at = now()
        WHERE key = $1
        RETURNING key, enabled, value
        "#,
    )
    .bind(&key)
    .bind(body.enabled)
    .bind(&body.value)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("flag".into()))?;

    sqlx::query(
        r#"INSERT INTO audit_logs (user_id, action, detail) VALUES ($1, 'flag.update', $2)"#,
    )
    .bind(admin.0.id)
    .bind(serde_json::json!({ "key": key, "enabled": row.enabled, "value": row.value }))
    .execute(&state.pool)
    .await?;

    Ok(Json(row))
}
