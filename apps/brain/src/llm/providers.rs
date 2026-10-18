use serde_json::{Value, json};

use crate::{
    error::{AppError, AppResult},
    llm::quota::ProviderName,
    llm::router::{ChatCompletion, ChatMessageIn},
    state::AppState,
};

pub async fn call_provider(
    state: &AppState,
    provider: ProviderName,
    messages: &[ChatMessageIn],
) -> AppResult<ChatCompletion> {
    match provider {
        ProviderName::Groq => call_groq(state, messages).await,
        ProviderName::Gemini => call_gemini(state, messages).await,
        ProviderName::Openrouter => call_openrouter(state, messages).await,
    }
}

async fn call_groq(state: &AppState, messages: &[ChatMessageIn]) -> AppResult<ChatCompletion> {
    let key = state
        .config
        .groq_api_key
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("groq not configured".into()))?;

    let body = json!({
        "model": "llama-3.3-70b-versatile",
        "messages": messages,
        "temperature": 0.4,
    });

    let resp = state
        .http
        .post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Anyhow(e.into()))?;

    handle_openai_compatible(resp, ProviderName::Groq).await
}

async fn call_openrouter(state: &AppState, messages: &[ChatMessageIn]) -> AppResult<ChatCompletion> {
    let key = state
        .config
        .openrouter_api_key
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("openrouter not configured".into()))?;

    let body = json!({
        "model": "openrouter/auto",
        "messages": messages,
        "temperature": 0.4,
    });

    let resp = state
        .http
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(key)
        .header("HTTP-Referer", "https://jarvis.local")
        .header("X-Title", "JarvisAI")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Anyhow(e.into()))?;

    handle_openai_compatible(resp, ProviderName::Openrouter).await
}

async fn call_gemini(state: &AppState, messages: &[ChatMessageIn]) -> AppResult<ChatCompletion> {
    let key = state
        .config
        .gemini_api_key
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("gemini not configured".into()))?;

    let mut contents = Vec::new();
    let mut system = String::new();
    for m in messages {
        if m.role == "system" {
            if !system.is_empty() {
                system.push('\n');
            }
            system.push_str(&m.content);
            continue;
        }
        let role = if m.role == "assistant" { "model" } else { "user" };
        contents.push(json!({
            "role": role,
            "parts": [{ "text": m.content }]
        }));
    }

    let mut body = json!({ "contents": contents });
    if !system.is_empty() {
        body["systemInstruction"] = json!({ "parts": [{ "text": system }] });
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={key}"
    );

    let resp = state
        .http
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Anyhow(e.into()))?;

    let status = resp.status();
    let value: Value = resp.json().await.map_err(|e| AppError::Anyhow(e.into()))?;
    if status.as_u16() == 429 {
        return Err(AppError::RateLimited);
    }
    if !status.is_success() {
        let msg = value
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("gemini error");
        return Err(AppError::Anyhow(anyhow::anyhow!("{msg}")));
    }

    let content = value
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(ChatCompletion {
        content,
        provider: ProviderName::Gemini,
    })
}

async fn handle_openai_compatible(
    resp: reqwest::Response,
    provider: ProviderName,
) -> AppResult<ChatCompletion> {
    let status = resp.status();
    let value: Value = resp.json().await.map_err(|e| AppError::Anyhow(e.into()))?;
    if status.as_u16() == 429 {
        return Err(AppError::RateLimited);
    }
    if !status.is_success() {
        let msg = value
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("provider error");
        return Err(AppError::Anyhow(anyhow::anyhow!("{msg}")));
    }
    let content = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(ChatCompletion { content, provider })
}
