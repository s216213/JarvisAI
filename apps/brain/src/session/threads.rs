use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
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
pub struct ThreadRow {
    pub id: Uuid,
    pub title: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MessageRow {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub role: String,
    pub content: String,
    pub provider: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct CreateThread {
    pub title: Option<String>,
}

pub fn threads_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_threads).post(create_thread))
        .route("/{id}/messages", get(list_messages))
}

async fn list_threads(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<ThreadRow>>> {
    let rows = sqlx::query_as::<_, ThreadRow>(
        r#"
        SELECT id, title, created_at, updated_at
        FROM threads WHERE user_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

async fn create_thread(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateThread>,
) -> AppResult<Json<ThreadRow>> {
    let title = body
        .title
        .unwrap_or_else(|| "New chat".to_string());
    let row = sqlx::query_as::<_, ThreadRow>(
        r#"
        INSERT INTO threads (user_id, title)
        VALUES ($1, $2)
        RETURNING id, title, created_at, updated_at
        "#,
    )
    .bind(user.id)
    .bind(title)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(row))
}

async fn list_messages(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<MessageRow>>> {
    let owned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM threads WHERE id = $1 AND user_id = $2)",
    )
    .bind(id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    if !owned {
        return Err(AppError::NotFound("thread".into()));
    }

    let rows = sqlx::query_as::<_, MessageRow>(
        r#"
        SELECT id, thread_id, role::text as role, content, provider, created_at
        FROM messages WHERE thread_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}
