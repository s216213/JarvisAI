use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{TokenKind, verify_token},
    error::AppError,
    llm::{
        complete_chat,
        router::{ChatMessageIn, mock_completion_if_no_keys},
    },
    memory::{build_memory_context, maybe_update_summary},
    persona::load_active_prompt,
    state::AppState,
    tools::{dispatch_tool, maybe_route_tool},
};

#[derive(Deserialize)]
pub struct WsQuery {
    pub ticket: String,
}

pub async fn chat_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(q): Query<WsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let claims = verify_token(&state.config.jwt_secret, &q.ticket)?;
    if claims.kind != TokenKind::WsTicket && claims.kind != TokenKind::Access {
        return Err(AppError::Unauthorized);
    }
    Ok(ws.on_upgrade(move |socket| handle_chat(socket, state, claims.sub)))
}

async fn handle_chat(socket: WebSocket, state: AppState, user_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        let Message::Text(text) = msg else { continue };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            let _ = sender
                .send(Message::Text(
                    json!({"type":"error","code":"bad_json","message":"invalid json"}).to_string().into(),
                ))
                .await;
            continue;
        };

        if value.get("type").and_then(|v| v.as_str()) != Some("chat.user") {
            continue;
        }

        let thread_id = match value
            .get("thread_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
        {
            Some(id) => id,
            None => {
                let _ = sender
                    .send(Message::Text(
                        json!({"type":"error","code":"bad_thread","message":"thread_id required"})
                            .to_string()
                            .into(),
                    ))
                    .await;
                continue;
            }
        };

        let content = value
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if content.is_empty() {
            continue;
        }

        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM threads WHERE id = $1 AND user_id = $2)",
        )
        .bind(thread_id)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(false);

        if !owned {
            let _ = sender
                .send(Message::Text(
                    json!({"type":"error","code":"not_found","message":"thread not found"})
                        .to_string()
                        .into(),
                ))
                .await;
            continue;
        }

        // Soft daily cap
        if let Ok(Some(cap)) = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT value FROM feature_flags WHERE key = 'llm.daily_soft_cap' AND enabled",
        )
        .fetch_optional(&state.pool)
        .await
        {
            let cap_n = cap.as_i64().unwrap_or(200);
            let used: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM messages m
                JOIN threads t ON t.id = m.thread_id
                WHERE t.user_id = $1 AND m.role = 'assistant' AND m.created_at > now() - interval '1 day'
                "#,
            )
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);
            if used >= cap_n {
                let _ = sender
                    .send(Message::Text(
                        json!({
                            "type":"error",
                            "code":"daily_cap",
                            "message":"Daily assistant reply soft-cap reached"
                        })
                        .to_string()
                        .into(),
                    ))
                    .await;
                continue;
            }
        }

        let _ = sqlx::query(
            r#"INSERT INTO messages (thread_id, role, content) VALUES ($1, 'user', $2)"#,
        )
        .bind(thread_id)
        .bind(&content)
        .execute(&state.pool)
        .await;

        let _ = sqlx::query("UPDATE threads SET updated_at = now(), title = CASE WHEN title = 'New chat' THEN left($2, 60) ELSE title END WHERE id = $1")
            .bind(thread_id)
            .bind(&content)
            .execute(&state.pool)
            .await;

        // Tool shortcut
        if let Some((tool, args)) = maybe_route_tool(&content) {
            match dispatch_tool(&state, user_id, tool, args).await {
                Ok(result) => {
                    let reply = format!("Tool `{tool}` result:\n{}", serde_json::to_string_pretty(&result).unwrap_or_default());
                    let msg_id = persist_assistant(&state, thread_id, &reply, None).await;
                    stream_text(&mut sender, thread_id, &reply, "local", msg_id).await;
                    continue;
                }
                Err(e) => {
                    let _ = sender
                        .send(Message::Text(
                            json!({"type":"error","code":"tool","message": e.to_string()})
                                .to_string()
                                .into(),
                        ))
                        .await;
                    continue;
                }
            }
        }

        let persona = load_active_prompt(&state).await.unwrap_or_else(|_| {
            "You are Jarvis, a precise personal AI assistant.".into()
        });
        let memory = build_memory_context(&state, user_id, thread_id)
            .await
            .unwrap_or_default();

        let history = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT role::text, content FROM messages
            WHERE thread_id = $1
            ORDER BY created_at DESC
            LIMIT 20
            "#,
        )
        .bind(thread_id)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        let mut messages = vec![ChatMessageIn {
            role: "system".into(),
            content: format!("{persona}\n\n{memory}"),
        }];
        for (role, content) in history.into_iter().rev() {
            messages.push(ChatMessageIn { role, content });
        }

        let completion = if let Some(mock) = mock_completion_if_no_keys(&state, &content) {
            Ok(mock)
        } else {
            complete_chat(&state, &messages).await
        };

        match completion {
            Ok(c) => {
                let provider = c.provider.as_str();
                let msg_id = persist_assistant(&state, thread_id, &c.content, Some(provider)).await;
                stream_text(&mut sender, thread_id, &c.content, provider, msg_id).await;
                let _ = maybe_update_summary(&state, thread_id).await;
            }
            Err(AppError::ProvidersExhausted { retry_after_secs }) => {
                let _ = sender
                    .send(Message::Text(
                        json!({
                            "type": "llm.providers_exhausted",
                            "retry_after_secs": retry_after_secs,
                            "message": "All LLM providers are temporarily unavailable. Please retry shortly."
                        })
                        .to_string()
                        .into(),
                    ))
                    .await;
            }
            Err(e) => {
                let _ = sender
                    .send(Message::Text(
                        json!({"type":"error","code":"llm","message": e.to_string()})
                            .to_string()
                            .into(),
                    ))
                    .await;
            }
        }
    }
}

async fn persist_assistant(
    state: &AppState,
    thread_id: Uuid,
    content: &str,
    provider: Option<&str>,
) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO messages (thread_id, role, content, provider)
        VALUES ($1, 'assistant', $2, $3)
        RETURNING id
        "#,
    )
    .bind(thread_id)
    .bind(content)
    .bind(provider)
    .fetch_one(&state.pool)
    .await
    .unwrap_or_else(|_| Uuid::new_v4())
}

async fn stream_text(
    sender: &mut (impl SinkExt<Message> + Unpin),
    thread_id: Uuid,
    content: &str,
    provider: &str,
    message_id: Uuid,
) {
    // Chunk for UI streaming feel
    for chunk in content.as_bytes().chunks(48) {
        let delta = String::from_utf8_lossy(chunk);
        let _ = sender
            .send(Message::Text(
                json!({
                    "type": "chat.assistant.delta",
                    "thread_id": thread_id,
                    "delta": delta,
                    "provider": provider
                })
                .to_string()
                .into(),
            ))
            .await;
    }
    let _ = sender
        .send(Message::Text(
            json!({
                "type": "chat.assistant.done",
                "thread_id": thread_id,
                "message_id": message_id,
                "provider": provider
            })
            .to_string()
            .into(),
        ))
        .await;
}
