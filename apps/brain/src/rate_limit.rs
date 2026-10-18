use std::collections::HashMap;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use parking_lot::Mutex;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Default)]
pub struct RateLimiter {
    hits: Mutex<HashMap<(Uuid, String), Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&self, user_id: Uuid, bucket: &str, limit: usize, window: Duration) -> bool {
        let mut map = self.hits.lock();
        let key = (user_id, bucket.to_string());
        let now = Instant::now();
        let entry = map.entry(key).or_default();
        entry.retain(|t| now.duration_since(*t) < window);
        if entry.len() >= limit {
            return false;
        }
        entry.push(now);
        true
    }
}

pub async fn llm_user_rate_limit(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    // Lightweight guard for REST; chat WS enforces daily soft-cap separately.
    if let Some(auth) = req.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(raw) = auth.to_str() {
            if let Some(token) = raw.strip_prefix("Bearer ") {
                if let Ok(claims) = crate::auth::verify_token(&state.config.jwt_secret, token) {
                    if !state
                        .rate_limiter
                        .check(claims.sub, "rest", 120, Duration::from_secs(60))
                    {
                        return Err(StatusCode::TOO_MANY_REQUESTS);
                    }
                }
            }
        }
    }
    Ok(next.run(req).await)
}
