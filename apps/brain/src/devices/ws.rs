use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    auth::{TokenKind, verify_token},
    devices::registry::DeviceInfo,
    error::AppError,
    state::AppState,
};

#[derive(Deserialize)]
pub struct WsQuery {
    pub ticket: String,
}

pub async fn device_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(q): Query<WsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let claims = verify_token(&state.config.jwt_secret, &q.ticket)?;
    if claims.kind != TokenKind::WsTicket && claims.kind != TokenKind::Access {
        return Err(AppError::Unauthorized);
    }
    let user_id = claims.sub;
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user_id)))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let mut device_id: Option<String> = None;

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            let Message::Text(text) = msg else { continue };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            let typ = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match typ {
                "device.hello" => {
                    let id = value
                        .get("device_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if id.is_empty() {
                        continue;
                    }
                    let platform = value
                        .get("platform")
                        .and_then(|v| v.as_str())
                        .unwrap_or("web")
                        .to_string();
                    let capabilities = value
                        .get("capabilities")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let label = value
                        .get("label")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let info = DeviceInfo {
                        device_id: id.clone(),
                        user_id,
                        platform: platform.clone(),
                        capabilities: capabilities.clone(),
                        label: label.clone(),
                        online: true,
                        last_seen: Utc::now(),
                    };

                    let now = Utc::now();
                    let caps_json = serde_json::json!(capabilities);
                    let _ = sqlx::query(
                        r#"
                        INSERT INTO devices (device_id, user_id, platform, label, capabilities, last_seen)
                        VALUES ($1, $2, $3, $4, $5, $6)
                        ON CONFLICT (device_id) DO UPDATE SET
                            platform = EXCLUDED.platform,
                            label = EXCLUDED.label,
                            capabilities = EXCLUDED.capabilities,
                            last_seen = EXCLUDED.last_seen
                        "#,
                    )
                    .bind(&id)
                    .bind(user_id)
                    .bind(&platform)
                    .bind(&label)
                    .bind(&caps_json)
                    .bind(now)
                    .execute(&state_clone.pool)
                    .await;

                    state_clone.devices.register(info, tx.clone());
                    device_id = Some(id);

                    let list = state_clone.devices.list_for_user(user_id);
                    let _ = tx.send(
                        json!({ "type": "device.list", "devices": list }).to_string(),
                    );
                }
                "device.heartbeat" => {
                    if let Some(id) = value.get("device_id").and_then(|v| v.as_str()) {
                        state_clone.devices.heartbeat(id);
                        let _ = sqlx::query("UPDATE devices SET last_seen = $2 WHERE device_id = $1")
                            .bind(id)
                            .bind(Utc::now())
                            .execute(&state_clone.pool)
                            .await;
                    }
                }
                "tool.result" => {
                    // Fan-in to user event bus for chat loop correlation.
                    state_clone.publish_user(user_id, text.to_string());
                }
                _ => {}
            }
        }
        device_id
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        res = &mut recv_task => {
            send_task.abort();
            if let Ok(Some(id)) = res {
                state.devices.unregister(&id);
            }
        }
    }
}
