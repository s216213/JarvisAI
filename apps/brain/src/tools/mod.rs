use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub capability: &'static str,
    pub trust: &'static str,
}

pub const CATALOG: &[ToolSpec] = &[
    ToolSpec {
        name: "web.notes.create",
        description: "Create a short note stored in memory facts",
        capability: "web.notes.create",
        trust: "safe",
    },
    ToolSpec {
        name: "web.memory.list",
        description: "List known user facts",
        capability: "web.memory.list",
        trust: "safe",
    },
    ToolSpec {
        name: "web.devices.list",
        description: "List online devices for the user",
        capability: "web.devices.list",
        trust: "safe",
    },
];

pub async fn execute_local_tool(
    state: &AppState,
    user_id: Uuid,
    tool: &str,
    args: &Value,
) -> AppResult<Value> {
    match tool {
        "web.notes.create" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("note");
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("value required".into()))?;
            sqlx::query(
                r#"
                INSERT INTO user_facts (id, user_id, key, value, updated_at)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (user_id, key) DO UPDATE SET
                    value = EXCLUDED.value,
                    updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(key)
            .bind(value)
            .bind(chrono::Utc::now())
            .execute(&state.pool)
            .await?;
            Ok(json!({ "ok": true, "key": key }))
        }
        "web.memory.list" => {
            let facts = sqlx::query_as::<_, (String, String)>(
                "SELECT key, value FROM user_facts WHERE user_id = $1 ORDER BY updated_at DESC LIMIT 50",
            )
            .bind(user_id)
            .fetch_all(&state.pool)
            .await?;
            Ok(json!({ "facts": facts.into_iter().map(|(k,v)| json!({"key": k, "value": v})).collect::<Vec<_>>() }))
        }
        "web.devices.list" => {
            let devices = state.devices.list_for_user(user_id);
            Ok(json!({ "devices": devices }))
        }
        _ => Err(AppError::BadRequest(format!("unknown local tool: {tool}"))),
    }
}

/// Very lightweight intent → tool routing for MVP (no tool-calling model required).
pub fn maybe_route_tool(user_text: &str) -> Option<(&'static str, Value)> {
    let lower = user_text.to_lowercase();
    if lower.contains("list devices") || lower.contains("what devices") {
        return Some(("web.devices.list", json!({})));
    }
    if lower.contains("list facts") || lower.contains("what do you know about me") {
        return Some(("web.memory.list", json!({})));
    }
    if let Some(rest) = lower.strip_prefix("remember that ") {
        return Some((
            "web.notes.create",
            json!({ "key": "note", "value": rest }),
        ));
    }
    None
}

pub async fn dispatch_tool(
    state: &AppState,
    user_id: Uuid,
    tool: &str,
    args: Value,
) -> AppResult<Value> {
    let spec = CATALOG.iter().find(|t| t.name == tool);
    let Some(spec) = spec else {
        return Err(AppError::BadRequest(format!("unknown tool {tool}")));
    };

    // Prefer online device with capability; fall back to local brain execution for web.* tools.
    if let Some(device) = state.devices.find_for_capability(user_id, spec.capability) {
        let invoke_id = Uuid::new_v4();
        let payload = json!({
            "type": "tool.invoke",
            "invoke_id": invoke_id,
            "tool": tool,
            "args": args,
            "device_id": device.device_id,
            "trust": spec.trust,
        });
        if state
            .devices
            .send_to_device(&device.device_id, payload.to_string())
        {
            // For MVP, also execute locally for web tools so chat remains responsive
            // even if the web device does not echo tool.result.
            if tool.starts_with("web.") {
                return execute_local_tool(state, user_id, tool, &args).await;
            }
            return Ok(json!({
                "dispatched": true,
                "invoke_id": invoke_id,
                "device_id": device.device_id
            }));
        }
    }

    if tool.starts_with("web.") {
        return execute_local_tool(state, user_id, tool, &args).await;
    }

    Err(AppError::BadRequest(format!(
        "No online device with capability '{}' for tool '{}'",
        spec.capability, tool
    )))
}
