use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::{
    error::{AppError, AppResult},
    llm::{
        providers::call_provider,
        quota::{ProviderName, QuotaGuardian},
    },
    state::AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageIn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ChatCompletion {
    pub content: String,
    pub provider: ProviderName,
}

pub async fn complete_chat(
    state: &AppState,
    messages: &[ChatMessageIn],
) -> AppResult<ChatCompletion> {
    let order = load_provider_order(state).await?;
    let mut last_err: Option<AppError> = None;

    for name in order {
        if !provider_configured(state, name) {
            continue;
        }
        if !state.quota.is_available(name) {
            continue;
        }

        let started = Instant::now();
        match call_provider(state, name, messages).await {
            Ok(completion) => {
                state
                    .quota
                    .mark_success(name, started.elapsed().as_millis() as u64);
                return Ok(completion);
            }
            Err(AppError::RateLimited) => {
                state.quota.mark_error(name, true);
                last_err = Some(AppError::RateLimited);
            }
            Err(e) => {
                state.quota.mark_error(name, false);
                last_err = Some(e);
            }
        }
    }

    Err(match last_err {
        Some(AppError::RateLimited) | None => AppError::ProvidersExhausted {
            retry_after_secs: state.quota.retry_after_secs(),
        },
        Some(e) => e,
    })
}

async fn load_provider_order(state: &AppState) -> AppResult<Vec<ProviderName>> {
    let value: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT value FROM feature_flags WHERE key = 'llm.order' AND enabled")
            .fetch_optional(&state.pool)
            .await?;

    let default = vec![
        ProviderName::Groq,
        ProviderName::Gemini,
        ProviderName::Openrouter,
    ];

    let Some(value) = value else {
        return Ok(default);
    };

    let Some(arr) = value.as_array() else {
        return Ok(default);
    };

    let parsed: Vec<ProviderName> = arr
        .iter()
        .filter_map(|v| v.as_str().and_then(ProviderName::parse))
        .collect();

    if parsed.is_empty() {
        Ok(default)
    } else {
        Ok(parsed)
    }
}

fn provider_configured(state: &AppState, name: ProviderName) -> bool {
    match name {
        ProviderName::Groq => state.config.groq_api_key.is_some(),
        ProviderName::Gemini => state.config.gemini_api_key.is_some(),
        ProviderName::Openrouter => state.config.openrouter_api_key.is_some(),
    }
}

pub fn mock_completion_if_no_keys(state: &AppState, user_text: &str) -> Option<ChatCompletion> {
    let any = state.config.groq_api_key.is_some()
        || state.config.gemini_api_key.is_some()
        || state.config.openrouter_api_key.is_some();
    if any {
        return None;
    }
    Some(ChatCompletion {
        content: format!(
            "Acknowledged. (dev mode — no LLM API keys configured.) You said: {user_text}"
        ),
        provider: ProviderName::Groq,
    })
}

pub fn guardian_snapshot(quota: &QuotaGuardian) -> Vec<crate::llm::quota::ProviderHealth> {
    quota.snapshot()
}
